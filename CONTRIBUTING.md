# Contributing to ferrigura

ferrigura is a port of Mermaid 12.2.0 to Rust. The port's plan, its decisions and its status are in [docs-port/](./docs-port/README.md); read [STATUS.md](./docs-port/STATUS.md) before you start, and the decision records in [docs-port/decisions/](./docs-port/decisions/) before you change something they settle. A new decision that is hard to reverse gets the next numbered record there.

## The reference

`packages/mermaid` and `packages/parser` are Mermaid 12.2.0 as this repository's fork of it. They are the reference the port is checked against, and they do not change: a difference from the reference is fixed in the port, or recorded where the port differs on purpose.

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

A change to a parser is checked with `pnpm port:diff <type>`, a change to a renderer with `pnpm port:geometry <type>` as well. A change in shared code must not change the output of the other diagram types; [tools/port/README.md](./tools/port/README.md) lists the commands.

## Commit messages

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/): `feat(…)` and `fix(…)` go into the changelog when release-please makes the next release ([docs-port/RELEASE.md](./docs-port/RELEASE.md)); `docs`, `chore`, `ci` and `test` do not.

## Conduct

Everyone taking part follows the [Code of Conduct](./CODE_OF_CONDUCT.md).
