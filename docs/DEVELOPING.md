# Developing ferrigura

How to set up the repository, run the checks, and make the common kinds of change. The rules behind it are [0003](./decisions/0003-conventional-commits.md) (commit messages) and [0005](./decisions/0005-quality-gates.md) (what every change must pass).

## Setup

- **Rust:** the toolchain is pinned by `rust-toolchain.toml` (stable, with clippy and rustfmt); rustup installs it on first use.
- **Node.js and pnpm:** the harness in `tools/port` needs Node.js 22.12 or newer and the pnpm version its `package.json` pins. With corepack (`corepack enable`), run pnpm from inside `tools/port`, where corepack finds that version; a pnpm installed on its own switches to it either way. The repository's root holds no JavaScript project, so every harness command runs with `-C tools/port` or from inside it.
- **The reference:** `tools/reference/setup.sh` builds upstream Mermaid at the pin with its patches into `target/reference/<commit>`, in about half a minute the first time ([0014](./decisions/0014-upstream-reference-at-a-pin.md)). The harness takes it from there, or from `REFERENCE_ROOT`.
- **Chromium,** for the geometry runs: `pnpm -C tools/port exec playwright install chromium`.

```bash
pnpm -C tools/port install
```

```bash
tools/reference/setup.sh
```

`pnpm -C tools/port install` also installs the pre-commit hook, which formats and lints the staged files.

## The checks

What CI runs on every change ([0005](./decisions/0005-quality-gates.md)):

```bash
cargo fmt --all --check
```

```bash
cargo clippy --all-targets -- -D warnings
```

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace
```

```bash
cargo test --workspace
```

```bash
pnpm -C tools/port run lint
```

```bash
pnpm -C tools/port test
```

On an aarch64 machine (CI runs it on macOS), the output of the whole corpus is pinned by hashes:

```bash
cargo test --release --test snapshots
```

## Checking a change against the reference

- **A parser change:** `pnpm -C tools/port port:diff <type>` compares the canonical models of ferrigura and the reference on every input of the type.
- **A renderer change:** `pnpm -C tools/port port:geometry <type>` renders the fixtures and probes with both in Chromium and compares the geometry; `--filter <text>` narrows the fixtures and `--images <dir>` writes side-by-side PNGs.
- **A change to shared code** (text, the unified renderer, configuration) must not move the output of other types: run the snapshots, and `tools/snapshots/diff.sh` shows which renders moved.
- **A change that moves the output on purpose** rewrites the snapshots in the same commit:

```bash
UPDATE_SNAPSHOTS=1 cargo test --release --test snapshots
```

[tools/port/README.md](../tools/port/README.md) lists every command of the harness, the run types, and the scripts that regenerate the corpora and fixtures.

## A difference from the reference

Every difference has one place ([0014](./decisions/0014-upstream-reference-at-a-pin.md)):

- ferrigura is wrong: fix ferrigura.
- ferrigura is behind a newer Mermaid: a `pending` entry in `tools/port/exceptions.ts`, with the issue that tracks the fix.
- Mermaid is wrong: a patch under `tools/reference/patches/`, a diff against the pinned commit with the earlier patches applied, whose header says what it fixes ([tools/reference/README.md](../tools/reference/README.md)).
- ferrigura leaves it out on purpose: an `unsupported` entry with its reason.

## The Node.js packages

From `node/`:

```bash
pnpm install --frozen-lockfile --ignore-scripts
```

```bash
pnpm build
```

Then `pnpm test`, `pnpm typecheck`, `pnpm lint`, `pnpm format:check` and `pnpm pack:check`; the last installs the packed facade and the host's platform package into a fresh project. A local run exercises only the host's binary; CI builds and checks all seven targets ([0016](./decisions/0016-npm-packages-via-napi.md)).

## Releases

release-please opens a release pull request from the commit messages on `main`; merging it tags `v<version>` and publishes the crates and npm packages ([0004](./decisions/0004-one-version-one-release.md)). Nothing else is needed for a release. To retry a failed publish, run `publish.yml` by hand with the release's tag.
