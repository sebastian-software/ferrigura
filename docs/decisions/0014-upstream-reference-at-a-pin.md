# 0014 The reference is upstream Mermaid at a pin

**Scope:** what ferrigura is checked against, how that reference is built, and where each difference between the two is recorded. It does not cover how the comparison itself works ([0015](./0015-differential-testing.md)).

## Decision

The reference is upstream Mermaid at a pinned commit, with a few patches. `tools/reference/pin` names the commit and the newest Mermaid release it contains; `tools/reference/patches/` holds the patches, applied in the order of their names; `tools/reference/setup.sh` fetches the commit, applies the patches and builds it in `target/reference/<commit>`.

Every difference between the reference and ferrigura has exactly one place:

- **ferrigura is behind Mermaid:** a `pending` exception in `tools/port/exceptions.ts`, which names the issue that tracks the fix.
- **Mermaid has a bug that ferrigura does not:** a patch on the pinned commit, whose header says what it fixes and, where there is one, names the upstream issue. The patch goes when the pin moves past upstream's fix.
- **ferrigura leaves something out on purpose**, such as HTML labels ([0009](./0009-svg-text-labels.md)), the dagre layout or configuration options that no code reads: a rule of the comparison or an `unsupported` exception, with its reason.

## Why

ferrigura promises to render what Mermaid renders, and only Mermaid itself can check that promise. The pin also gives ferrigura a way to follow Mermaid: moving it to a newer release shows, as differences, what ferrigura has to catch up on, and brings in that release's tests and examples as inputs.

Weighed and not chosen: a fork of Mermaid as the reference, which checks ferrigura against its own changes and gives no path to newer releases; and the reference's answers frozen as golden files, which are tied for good to one version of Mermaid and bring in none of the tests and examples of newer releases.

## Consequences

- The reference is not in the repository. CI builds it once per pin and patch set and caches the build.
- Renovate opens one pull request per Mermaid minor release that moves the pin, never merged automatically: moving it can need rebased patches and new pending exceptions ([tools/reference/README.md](../../tools/reference/README.md)).
- A run fails on a difference that nothing covers, on a pending exception without an issue, and on an exception that covers nothing any more, so the list cannot outlive the difference it records.

## Related

[0015](./0015-differential-testing.md), [0005](./0005-quality-gates.md).
