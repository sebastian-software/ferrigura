# Check the port against upstream Mermaid at a pinned commit

The JS reference is upstream Mermaid at a pinned commit, with a few patches, not a fork of our own. ferrigura promises to render what Mermaid renders, and only Mermaid itself can check that promise. The pin also gives the port a way to follow Mermaid: moving it to a newer release shows, as differences, what ferrigura has to catch up on. The differential runs stay as [0003](./0003-differential-testing.md) describes them; only their reference changes, from the fork of Mermaid 12.2.0 that this repository held.

Every difference between the reference and ferrigura has one place:

- **ferrigura is behind Mermaid:** a pending exception that names the issue tracking the fix.
- **Mermaid has a bug that ferrigura does not:** a patch on the pinned commit, which names the bug and, where there is one, the upstream issue. The patch goes when the pin moves past upstream's fix.
- **ferrigura leaves something out on purpose**, such as HTML labels ([0004](./0004-svg-text-labels.md)), the dagre layout or configuration options that no code reads: a rule of the comparison or an unsupported exception, with its reason.

A run stops on a difference that nothing covers, on a pending exception without an issue, and on an exception that covers nothing any more.

The pin stayed at 12.2.0 (`e28fbd9de`), where the fork started, until the fork left the repository. From then on it follows Mermaid's minor releases, one pull request per release.

Considered and not chosen: keeping the fork as the reference, which checks ferrigura against our own changes to Mermaid and gives no path to newer releases; and freezing the reference's answers as golden files, about 12 MB compressed in the repository, or the same files on a static host or as a CI artifact. Golden files are fixed for good to one version of Mermaid, and none of them brings in the tests and examples of newer releases.

## Consequences

- The reference is not in the repository: `tools/reference/setup.sh` fetches the pinned commit, applies the patches and builds it, and CI caches the build per pin and patch set.
- ferrigura's own output is pinned separately, by one hash per corpus file in `crates/ferrigura/tests/snapshots/svg.tsv`, so a change of the output shows even where the differential runs do not look.
- The fork's last state is the tag `fork-mermaid-12.2.0`. The corpus capture tools that needed its dev server (ELK, cose-bilkent, fcose, swimlanes) are only there; a corpus they made is captured again only after its tool is ported to the pinned reference.
- Nothing in the repository needs Mermaid's history any more.
