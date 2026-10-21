# The first release of ferrigura

The plan for publishing ferrigura for the first time: what goes out, what has to be done before, how releases are made from then on, and the decisions that are Sebastian's. Status: released on 2026-10-21. ferrigura 0.1.0 is on crates.io: Sebastian published the twelve crates from the tag `ferrigura-v0.1.0` with `tools/release/publish-crates.sh`, and the workflow publishes the next release. Open: Firefox.

## What is released

| Channel             | What                                                                                                                       | First release                                                                         |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| crates.io           | The library `ferrigura` and the eleven crates it is built from; the command line tool `ferrigura-cli` (binary `ferrigura`) | Yes                                                                                   |
| npm                 | The Node addon (`crates/ferrigura-node`), with prebuilt binaries per platform                                              | Later: it needs a build matrix and a package name                                     |
| GitHub releases     | The command line tool as prebuilt binaries                                                                                 | Later; `cargo install ferrigura-cli` works from the first release                     |
| The JavaScript fork | `mermaid` and `@mermaid-js/parser` in `packages/`                                                                          | Not released from this repository: they are the reference the port is checked against |

All names are free on 2026-10-20: the thirteen crate names on crates.io and `ferrigura` and `@ferrigura/node` on npm (each answered 404).

The crates, in the order `cargo publish --workspace` uploads them (each after the crates it depends on):

| Crate                                                                      | Depends on (ferrigura crates)  | Licence                                                 |
| -------------------------------------------------------------------------- | ------------------------------ | ------------------------------------------------------- |
| `ferrigura-jsmath`                                                         | none                           | MIT AND Apache-2.0 WITH LLVM-exception AND BSD-3-Clause |
| `ferrigura-model`, `ferrigura-text`, `ferrigura-elk`                       | none                           | MIT; MIT, the faces OFL-1.1 and CC-BY-4.0; EPL-2.0      |
| `ferrigura-path`, `ferrigura-cose`, `ferrigura-venn`, `ferrigura-swimlane` | `ferrigura-jsmath`             | MIT AND BSD-3-Clause; MIT; MIT; MIT                     |
| `ferrigura-syntax`                                                         | `ferrigura-model`              | MIT                                                     |
| `ferrigura-render`                                                         | all of the above               | MIT                                                     |
| `ferrigura`                                                                | model, syntax, render          | MIT                                                     |
| `ferrigura-cli`                                                            | `ferrigura`, `ferrigura-model` | MIT                                                     |

`ferrigura-node` is a `cdylib` for Node and stays off crates.io.

## Versions

All crates share one version and are released together: a change in any crate is a release of all of them, which is how they are tested. The first version is `0.1.0`. Before `1.0.0`, a feature raises the minor version and a fix the patch version, as Cargo's rules for `0.x` versions read them: `0.2.0` is a breaking change for `0.1.x` users.

## Before the first publish

Done on 2026-10-20 and 2026-10-21:

| Step                                  | State                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Build inputs inside the crates**    | `ferrigura-model` reads its own copy of the configuration schema, `schema/config.schema.yaml`; the test `schema_copy` fails when the copy and `packages/mermaid/src/schemas/config.schema.yaml` differ. Every other crate reads only its own files (checked with `grep` for `../` in the build scripts and the `include_*!` macros).                                                                                                                                                                                                         |
| **Package contents**                  | Each publishable crate has an `include` list: `src`, its README, licence and notice files, and the model's build script and schema, the text crate's fonts. The packages, compressed: `ferrigura-text` 811 KB (the fonts), `ferrigura-render` 750 KB, `ferrigura-syntax` 476 KB, `ferrigura-elk` 299 KB, `ferrigura-model` 128 KB, `ferrigura-swimlane` 127 KB, `ferrigura-cose` 66 KB, `ferrigura-jsmath` 32 KB, `ferrigura-venn` 26 KB, `ferrigura` 19 KB, `ferrigura-path` 16 KB, `ferrigura-cli` 13 KB; all far below crates.io's 10 MB. |
| **Versions on the path dependencies** | Every dependency between the crates has `version = "0.0.0"` beside its path; release-please raises them with the crates.                                                                                                                                                                                                                                                                                                                                                                                                                     |
| **Metadata**                          | As the standards' Rust reference asks of a published crate: `readme`, `homepage` (the repository until there is a site), `documentation` on docs.rs, `keywords` with the family keyword `ferrigura`, `[package.metadata.docs.rs]`, and `[lints] workspace = true` with `unsafe_code = "deny"`.                                                                                                                                                                                                                                               |
| **Licences in the packages**          | The seven MIT crates without a licence file have the repository's `LICENSE`; the others keep their own (ELK's EPL-2.0, the notices of `ferrigura-jsmath` and `ferrigura-path`). `ferrigura-text` is `MIT AND OFL-1.1 AND CC-BY-4.0`, with the faces' licences and Twemoji's attribution in `fonts/` and `NOTICE.md`.                                                                                                                                                                                                                         |
| **Well-formed XML**                   | Merged on 2026-10-20: every SVG is well-formed XML, and a test parses every fixture's output.                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| **Browsers**                          | Safari 27 draws the colour emoji (spike `fallback-faces.md`, section 8).                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| **Docs**                              | `cargo doc` without warnings, checked in CI with `RUSTDOCFLAGS="-D warnings"`.                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| **Dry run**                           | `tools/release/publish-crates.sh --dry-run HEAD` checks the versions and the order and packages the twelve crates and builds each from its package (`cargo publish --dry-run`), in the order of their dependencies.                                                                                                                                                                                                                                                                                                                          |

The first release, on 2026-10-21:

1. **The release pull request**: release-please opened it ([#6](https://github.com/sebastian-software/ferrigura/pull/6)) with the organisation's `RELEASE_PLEASE_TOKEN`, and CI ran on it. The standards' proof held: one candidate, every member and internal requirement at `0.1.0`, `Cargo.lock` current, and the packages with the same files as before (`tools/release/publish-crates.sh --dry-run` on its head). The changelog has the features of the first release from one commit that lists them as several entries.
2. **The READMEs**: the twelve published crates no longer say that they are not published yet; changed before the tag, as the packages carry them.
3. **The tag**: merging the pull request created the tag `ferrigura-v0.1.0` and the GitHub release; `publish-crates` did not run.
4. **The publish**, by Sebastian, with `tools/release/publish-crates.sh ferrigura-v0.1.0`: the first five crates went at once, the others one about every ten minutes, after crates.io's limit on new crates (HTTP 429), as expected.
5. **Trusted Publishing**, for each of the twelve crates in its settings on crates.io (GitHub, `sebastian-software/ferrigura`, workflow `publish.yml`, no environment), and the variable `PUBLISH_ENABLED`, `true` since 2026-10-21: the workflow publishes every later release.

Left: **Firefox** is not checked yet for the colour emoji.

The trademark check: Sebastian takes the name checks of 2026-10-21 as enough for now (crates.io, npm, PyPI, Homebrew, Docker Hub, GitHub and the domains free, no use of the name on the web); a formal search in the registers can follow.

## How releases are made

The release follows the product pattern of `sebastian-software/standards` (`reference/release-please/README.md`, `standards-v0.9.0`): the crates share one version, so they are one product with one release pull request, one tag, one changelog and one GitHub release.

- The repository's root `Cargo.toml` is the package `ferrigura`, the library; its sources stay in `crates/ferrigura`, with the examples and tests listed by path. `default-members` keeps plain cargo commands at the root covering the whole workspace.
- `release-please-config.json` has one component, `ferrigura` at `.`, with the `rust` release type. Its Rust strategy reads the root package's version and moves every member of the workspace, the versions of the dependencies between them and `Cargo.lock` with it; no `cargo-workspace` or `linked-versions` plugin. `.release-please-manifest.json` holds the one version. The changelog is `crates/ferrigura/CHANGELOG.md`, as the root `CHANGELOG.md` is Mermaid's. Tags read `ferrigura-v0.1.0`.
- The commit messages are the release notes: `feat(…)` and `fix(…)` as the repository writes them already. `bump-minor-pre-major` keeps a breaking change before `1.0.0` at a minor version.
- `bootstrap-sha` was the commit release-please started from, so that the first changelog did not list the whole history of the port. It went after the first release; release-please starts from the last release's tag.
- `.github/workflows/publish.yml` is the standards' publish skeleton: the job `release-please` on every push to `main`, and `publish-crates`, which runs from the release's tag when a release is created and publishes the crates in the order of their dependencies through the standards' `publish-crates` action (it waits for each version in the index, retries once and skips a version already published). A manual run with a release's tag retries a failed publish. Only `publish-crates` waits, for the variable `PUBLISH_ENABLED`.
- A pull request opened with the default `GITHUB_TOKEN` does not start the CI workflow, so release-please opens the release pull requests with the organisation's `RELEASE_PLEASE_TOKEN`, as the other repositories of the organisation do (`GITHUB_TOKEN` is the fallback).
- The workflow publishes with crates.io's Trusted Publishing (OpenID Connect) and needs no token. Trusted Publishing cannot create a crate, so the first release is published by hand with `tools/release/publish-crates.sh`, which publishes the same list in the same order as the workflow. The organisation's `CARGO_REGISTRY_TOKEN` is not used: it would publish only crates its account owns.

The changesets files, scripts and dependencies are removed; the JavaScript packages are not released from here.

## Decisions

Decided by Sebastian on 2026-10-20, as proposed:

1. The first release goes to crates.io only; npm and prebuilt binaries come later.
2. One version for all crates.
3. The first version is `0.1.0`.
4. `ferrigura-model` holds a copy of the schema, checked against the source.
5. A GitHub App's token opens the release pull requests. Replaced on 2026-10-21: the organisation's `RELEASE_PLEASE_TOKEN` opens them.

Decided by Sebastian on 2026-10-21: the release follows the product pattern and the publish skeleton of the standards, and the name checks of the day are enough as the trademark check for now. The release pull requests use the organisation's token instead of a GitHub App, and Sebastian publishes the first release himself, with a script and his own crates.io login.
