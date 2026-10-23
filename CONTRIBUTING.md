# Contributing to ferrigura

ferrigura is a port of Mermaid 12.2.0 to Rust. The port's plan, its decisions and its status are in [docs-port/](./docs-port/README.md); read [STATUS.md](./docs-port/STATUS.md) before you start, and the decision records in [docs-port/decisions/](./docs-port/decisions/) before you change something they settle. A new decision that is hard to reverse gets the next numbered record there.

## The reference

The port is checked against upstream Mermaid at a pinned commit, with a few patches ([tools/reference/](./tools/reference/README.md)): `tools/reference/setup.sh` builds it, and the harness takes it from there. A difference from the reference is fixed in the port, recorded as a pending or unsupported exception (`tools/port/exceptions.ts`), or, when Mermaid is wrong, fixed in the reference by a patch.

## Checks

Every change passes the checks CI runs:

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
pnpm test
```

On an aarch64 machine (CI runs it on macOS), the output of the whole corpus is pinned by hashes:

```bash
cargo test --release --test snapshots
```

A change that moves the output on purpose rewrites them with `UPDATE_SNAPSHOTS=1` in the same commit; `tools/snapshots/diff.sh` shows which renders moved.

A change to a parser is checked with `pnpm port:diff <type>`, a change to a renderer with `pnpm port:geometry <type>` as well. A change in shared code must not change the output of the other diagram types; [tools/port/README.md](./tools/port/README.md) lists the commands.

## Commit messages

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/): `feat(…)` and `fix(…)` go into the changelog when release-please makes the next release ([docs-port/RELEASE.md](./docs-port/RELEASE.md)); `docs`, `chore`, `ci` and `test` do not.

## Conduct

Everyone taking part follows the [Code of Conduct](./CODE_OF_CONDUCT.md).
