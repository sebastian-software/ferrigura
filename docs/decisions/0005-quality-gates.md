# 0005 Quality gates

**Scope:** what every change to `main` must pass, locally and in CI (`.github/workflows/ci.yml`). It does not cover what a release adds on top ([0004](./0004-one-version-one-release.md)).

## Decision

A change lands only with the whole suite green. Nothing is switched off to make it green.

- **Rust:** `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace` and `cargo test --workspace`. The workspace denies `unsafe_code`.
- **Harness and repository formatting:** `pnpm -C tools/port run lint` (tsc, oxlint, oxfmt over the repository's Markdown, JSON, YAML and TypeScript) and `pnpm -C tools/port test`.
- **Differential runs:** `pnpm -C tools/port port:diff <type>` for every model type and data run, and `pnpm -C tools/port port:geometry <type>` for every diagram type, against the reference ([0014](./0014-upstream-reference-at-a-pin.md), [0015](./0015-differential-testing.md)). A run fails on a difference that no exception covers, on a pending exception without an issue, and on an exception that covers nothing any more.
- **Snapshots:** `cargo test --release --test snapshots` pins the output of the whole corpus by hashes. CI runs it on macOS on aarch64, where the hashes were taken.
- **npm packages:** the native build matrix and the package checks of `node/` ([0016](./0016-npm-packages-via-napi.md)).

## Why

ferrigura promises that the same diagram text gives the same diagram as Mermaid. That promise holds only while every check that defines it runs on every change; a check that is skipped once stops meaning anything.

## Consequences

- A change that moves the output on purpose rewrites the snapshots with `UPDATE_SNAPSHOTS=1` in the same commit and says why; `tools/snapshots/diff.sh` shows which renders moved.
- A change in shared code must not change the output of the other diagram types. The snapshots show it if it does.
- A ported spec that cannot pass is marked `#[ignore = "port: <reason>"]`, not deleted, so that the count of ignored tests stays visible. Tests that only print diagnostics are ignored as `diagnostic:` or `logging:` and run by hand.
- The pre-commit hook (`tools/port/.husky`, lint-staged) formats and lints the staged files; it needs `pnpm` on the `PATH`.

## Related

[0003](./0003-conventional-commits.md), [0015](./0015-differential-testing.md), [DEVELOPING.md](../DEVELOPING.md) for the commands.
