# 0013 Licenses of ported code

**Scope:** the licenses under which ferrigura's crates and npm packages are published, and how code and fonts ported or bundled from other sources are credited. It does not cover the licenses of ferrigura's dependencies, which Cargo and npm record.

## Decision

ferrigura is MIT-licensed, as Mermaid is: it ports Mermaid, and Mermaid's copyright notice stays in `LICENSE` beside ferrigura's. Code ported from another source keeps that source's license:

| Crate                                                                        | License                                                 | Ported or bundled from                                                          |
| ---------------------------------------------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `ferrigura-elk`                                                              | EPL-2.0                                                 | ELK's Java sources                                                              |
| `ferrigura-jsmath`                                                           | MIT AND Apache-2.0 WITH LLVM-exception AND BSD-3-Clause | llvm-libc and V8 (fdlibm)                                                       |
| `ferrigura-path`                                                             | MIT AND BSD-3-Clause                                    | Chromium and Skia                                                               |
| `ferrigura-text`                                                             | MIT AND OFL-1.1 AND CC-BY-4.0                           | the bundled fonts and Twemoji                                                   |
| `ferrigura-cose`, `ferrigura-venn`, `ferrigura-swimlane`, `ferrigura-render` | MIT                                                     | MIT sources: cose-bilkent and fcose, venn.js, Mermaid's swimlane layout, marked |
| all others                                                                   | MIT                                                     | Mermaid                                                                         |

- A crate that ports or bundles anything carries a `NOTICE.md` that names each part, where it lives in the crate, its source and version, and its license, with the license texts beside it (`LICENSE-*`).
- Code under a license that is not compatible with MIT for redistribution in one crate lives in a crate of its own under that license: ELK's EPL-2.0 is the case, and a port is a derivative work.
- The npm packages carry the notices of everything their binary contains (`node/scripts/write-notices.mjs` writes `NOTICE.md`, and `pnpm -C node lint` checks it).

## Why

Porting is copying in another language; the authors of the sources keep their rights, and their licenses say how to credit them. A crate per license keeps every crate's license expression exact, and lets users who avoid a license (EPL-2.0) see what depends on it.

## Consequences

- A change that ports or bundles new code or a font updates the crate's `NOTICE.md` and, where the license differs, its `license` field, in the same change.
- Each published crate includes its `LICENSE`, `NOTICE.md` and `LICENSE-*` files ([0004](./0004-one-version-one-release.md)).

## Related

[0004](./0004-one-version-one-release.md), [0010](./0010-text-measurement-and-fonts.md), [0012](./0012-layout-engines-ported-exactly.md).
