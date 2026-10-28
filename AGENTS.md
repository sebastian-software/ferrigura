# Working in ferrigura

ferrigura renders Mermaid diagram text to SVG in Rust, without a browser. Read [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) for the map and [docs/DEVELOPING.md](./docs/DEVELOPING.md) for the commands.

The rules are the [decisions](./docs/decisions/README.md); these hold for every change:

- Write in US English: code, comments, docs, commit messages ([0001](./docs/decisions/0001-us-english.md)). Use the terms of the [glossary](./docs/decisions/0002-glossary.md).
- Commit messages follow Conventional Commits; `feat` and `fix` only for what the published packages show ([0003](./docs/decisions/0003-conventional-commits.md)).
- Keep every check green and switch none off; a change that moves the output rewrites the snapshots in the same commit and says why ([0005](./docs/decisions/0005-quality-gates.md)).
- Check parser changes with `port:diff` and renderer changes with `port:geometry` against the reference; record a difference only where [0014](./docs/decisions/0014-upstream-reference-at-a-pin.md) says.
- Docs describe the current state only: no dates, no history, no plans. Update the page a change makes wrong in the same change: a decision record, [LEARNINGS.md](./docs/LEARNINGS.md), or the type's page under [docs/diagrams/](./docs/diagrams/README.md).
- A new decision that is hard to reverse gets a decision record; something learned that the next person would otherwise have to find again goes into LEARNINGS.md or the type's page.

Run the harness with `pnpm -C tools/port …`; with corepack, run pnpm from inside `tools/port`. The pre-commit hook needs `pnpm` on the `PATH`.
