# The reference

The port is checked against upstream Mermaid at a pinned commit, with a few patches applied ([0010](../../docs-port/decisions/0010-upstream-reference-at-a-pin.md), [docs-port/ORACLE-PLAN.md](../../docs-port/ORACLE-PLAN.md)). This folder holds what makes that reference:

| File       | Contents                                                                                                                    |
| ---------- | --------------------------------------------------------------------------------------------------------------------------- |
| `pin`      | The upstream repository and the commit, one per line: `repository <url>` and `commit <sha>`                                 |
| `patches/` | The patches on that commit, applied in the order of their names. Each starts with why it exists, then the diff              |
| `setup.sh` | Fetches the pinned commit, applies the patches, installs and builds it in `target/reference/<commit>`, and prints that path |

```bash
REFERENCE_ROOT="$(tools/reference/setup.sh)" pnpm port:diff flowchart
```

The first run takes about half a minute; later runs reuse the build while the pin and the patches are unchanged. CI builds the reference the same way, once per pin and patch set, and caches the build. Without `REFERENCE_ROOT`, the harness takes this build of the pin.

## The patches

| Patch | What it fixes in upstream Mermaid 12.2.0                                                                      |
| ----- | ------------------------------------------------------------------------------------------------------------- |
| 01    | `topAxis` in a gantt chart failed to parse (`yy.TopAxis` does not exist)                                      |
| 02    | A diagram with the graduated `architecture` keyword was detected and then failed to parse                     |
| 03    | The `wardley-beta` configuration section never applied its `theme`, `look` and `layout`                       |
| 04    | `defaultConfig.ts` copied the flowchart section into `c4.flowchart`                                           |
| 05    | Front matter and directives dropped the options only railroad has                                             |
| 06    | `@{ shape: constructor }` was accepted as a shape, and `__proto__` twice in a subgraph stayed twice           |
| 07    | Not a fix: a capture hook, `globalThis.__mermaidElkCapture`, that gives the harness each ELK input and layout |
| 08    | The sankey, packet, xychart and block detectors took any word that starts with the keyword                    |
| 09    | With patch 02, `architecture` became a keyword everywhere; it is the header keyword only now                  |

Patches 01 to 06 are the fork's fixes (at the tag `fork-mermaid-12.2.0`), limited to `packages/` and `.build/`. A new patch is a diff against the pinned commit with the earlier patches applied, with a header that says what it fixes and, where there is one, the upstream issue. When upstream fixes a bug, its patch goes with the next move of the pin.

## Moving the pin

Change the commit in `pin`, run `setup.sh`, and see which patches no longer apply or are no longer needed. Then run the checks: every new difference is fixed in the same change or becomes a pending exception that names its issue (`tools/port/exceptions.ts`).
