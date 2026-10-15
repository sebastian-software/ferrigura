# ELK sources for `ferrigura-elk`

Which Java sources the port follows, and how they were pinned. The port has to match what elkjs computes, so the pin is the version elkjs was built from, not the latest ELK.

## What Mermaid runs

- Mermaid depends on `elkjs@^0.9.3` (`packages/mermaid/package.json`), and `pnpm-lock.yaml` locks it to `0.9.3`.
- The bundle Mermaid loads is `elkjs/lib/elk.bundled.js` (`layout-algorithms/elk/render.ts`).

## elkjs 0.9.3

| Item                 | Value                                                                                                                                 |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| npm package          | `elkjs@0.9.3`, tarball `https://registry.npmjs.org/elkjs/-/elkjs-0.9.3.tgz`                                                           |
| elkjs repository     | `https://github.com/kieler/elkjs`, tag `0.9.3`                                                                                        |
| elkjs release commit | `a8304cf79fde75bc2ab1a89d28320f53f8637436`, "add files for 0.9.3 release", 2024-04-16                                                 |
| elkjs parent         | `8ebfadac8`, merge of PR #274 (worker termination)                                                                                    |
| elkjs build          | `build.gradle` compiles ELK from a sibling checkout (`elkRepo = '../elk'`) with GWT 2.10.0; the commit does not record the ELK commit |

## ELK source used

| Item           | Value                                                                                                                                                                                        |
| -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository     | `https://github.com/eclipse/elk` (also served as `https://github.com/eclipse-elk/elk`)                                                                                                       |
| Commit         | `9bc93474e1dc649450e8c97abb45d16b7c88e35b`, "Metacompiler fix (#1024)", committed 2024-04-15 16:59 UTC                                                                                       |
| Local checkout | `.elk-src/elk` (gitignored by `/.elk-src/`), created with `git clone --filter=blob:none --no-checkout https://github.com/eclipse/elk .elk-src/elk && git -C .elk-src/elk checkout 9bc93474e` |
| License        | Eclipse Public License 2.0 (`LICENSE.md` in the repository)                                                                                                                                  |

`9bc93474e` is the head of ELK `master` on the day elkjs 0.9.3 was released. elkjs does not record the ELK commit it was built from, so this is an inference, with the evidence below.

### Why this commit

1. **Not the `v0.9.1` tag** (`62d5909`, 2024-04-05). The elkjs bundle contains the top-down change from ELK commit `7ca51784e` (#955, 2024-04-09): `NodeContext` gains a `topdownLayout` field, and `NodeSizeCalculator` uses it. The bundle has `this.topdownLayout = …` in the `NodeContext` constructor and `nodeContext.topdownLayout?(height = $wnd.Math.max(…))` in `setNodeHeight`/`setNodeWidth` (`lib/elk-worker.js` lines 22954, 23781, 23810). `v0.9.1` has no `topdownLayout` in `NodeContext.java`.
2. **Commits after `7ca51784e` change no runtime code.** Between `7ca51784e` and `9bc93474e`, ELK `master` has `e486c42c2` "Commit all generated layout options and providers (#1022)" and its revert `dcde05603` (net zero on the sources, since the revert restores all 36 files), plus documentation and templates. `9bc93474e` changes only the melk metacompiler tooling (`org.eclipse.elk.core.meta`).
3. **Head on the release day.** The newest `master` commit before the elkjs release commit (2024-04-16 08:20 UTC) is `9bc93474e`.

The top-down change does not reach Mermaid, which never sets `elk.topdownLayout`. The commit only matters for fidelity, and the differential corpus compares against elkjs itself, so a mismatch would show up there.

## Metadata is generated

Option IDs, defaults and enum values are declared in `*.melk` files, not in Java: `plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/Core.melk`, `plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/Layered.melk` and `plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/labels/Labels.melk`. The melk metacompiler generates the `*Options` and `*MetaDataProvider` classes at build time. They are not in the repository at this commit (`e486c42c2` committed them and `dcde05603` reverted that). The port therefore takes defaults from the melk files.

## elkjs-specific glue (not in ELK)

The JSON contract (what goes in, what comes out, how `layoutOptions` keys are resolved) comes partly from elkjs itself, at `a8304cf`:

- `src/java-additional/org/eclipse/elk/graph/json/JsonAdapter.xtend`
- `src/java/org/eclipse/elk/js/ElkJs.java` and `src/java/org/eclipse/elk/js/linker/NodeJsModuleLinker.java`
- `src/js/elk-api.js`, `src/js/main-api.js`, `src/js/main-node.js`

Their text is read from the elkjs repository at that commit, not from the npm tarball, which only ships compiled output.

## Reproduce

```bash
git clone --filter=blob:none --no-checkout https://github.com/eclipse/elk .elk-src/elk
git -C .elk-src/elk checkout 9bc93474e1dc649450e8c97abb45d16b7c88e35b
```
