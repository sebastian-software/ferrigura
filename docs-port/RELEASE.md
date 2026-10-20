# The first release of ferrigura

The plan for publishing ferrigura for the first time: what goes out, what has to be done before, how releases are made from then on, and the decisions that are Sebastian's. Status: a plan, prepared on 2026-10-20. Nothing is published.

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

1. **Trademark check** for `ferrigura` (DPMA, EUIPO, USPTO), as `STATUS.md` has it. A crate name on crates.io cannot be given back, so this comes first.
2. **Build inputs inside the crates.** `ferrigura-model`'s `build.rs` reads `packages/mermaid/src/schemas/config.schema.yaml`, which is outside the crate and not in its package. The crate needs its own copy of the schema, kept equal to the source by a test or a CI check, so that the YAML stays the single source. Every other crate reads only its own files (checked with `grep` for `../` in the build scripts and the `include_*!` macros).
3. **Package contents.** crates.io takes at most 10 MB per crate. The tracked files of `ferrigura-swimlane` are 38.7 MB, of `ferrigura-cose` 11.9 MB, of `ferrigura-render` 9.6 MB and of `ferrigura-venn` 7.3 MB, almost all of it test corpora. Each crate gets an `include` list (its `src`, build script, fonts, licence and notice files, README) and is checked with `cargo package --list` and the size of the `.crate` file.
4. **Versions on the path dependencies.** Every dependency between the crates is a `path` dependency today; for crates.io it needs `version = "0.1.0"` beside the path. release-please keeps these versions up to date from then on.
5. **Metadata.** `publish = false` in `[workspace.package]` goes, per crate; each crate gets `readme`, and `ferrigura` and `ferrigura-cli` `keywords`, `categories` and `documentation`. The user documentation is in the crates' README files.
6. **Licences in the packages.** Each crate's licence files go into its package: the ELK crate's EPL-2.0, the notices of `ferrigura-jsmath` and `ferrigura-path`, and in `ferrigura-text` the OFL texts of the faces and Twemoji's CC BY 4.0 attribution (`crates/ferrigura-text/NOTICE.md`). The `license` field of `ferrigura-text` then names the faces' licences beside MIT, as an SPDX expression.
7. **Well-formed XML.** The branch `sebastian/zealous-sutherland-8c49a9` makes every SVG well-formed XML (the xlink prefix, C4's quoted face, entities); it is merged before the release.
8. **Browsers.** Safari 27 draws the colour emoji (spike `fallback-faces.md`, section 8). Firefox is not checked yet.
9. **A dry run**: `cargo publish --workspace --dry-run` and `cargo doc` as docs.rs builds it (`cargo +nightly doc` with `--cfg docsrs` only if a crate uses it).

## How releases are made

[release-please](https://github.com/googleapis/release-please) replaces the changesets setup inherited from Mermaid:

- `release-please-config.json` lists the twelve crates with the `rust` release type, the `cargo-workspace` plugin (it updates the versions of the dependencies between the crates) and the `linked-versions` plugin (one version for all). `.release-please-manifest.json` holds the current version of each crate.
- The commit messages are the release notes: `feat(…)` and `fix(…)` as the repository writes them already. Each crate gets a `CHANGELOG.md`.
- `bootstrap-sha` is the commit release-please starts from, so the first changelog does not list the whole history of the port; the first release is forced to `0.1.0` with a `Release-As: 0.1.0` footer when it is due.
- `.github/workflows/release-please.yml` runs release-please. Until the first release it runs only by hand (`workflow_dispatch`), so it opens no release pull request before the steps above are done; then it runs on every push to `main`.
- When a release pull request is merged, release-please tags the commit and makes a GitHub release. Publishing to crates.io is a job after that (`cargo publish --workspace`), added with the first release. crates.io's trusted publishing (OpenID Connect from GitHub Actions) needs no long-lived token, but it can be set up only for a crate that exists: the first version is published with a token of Sebastian's, then trusted publishing takes over.
- A pull request opened with the default `GITHUB_TOKEN` does not start the CI workflow. The release pull requests need a token of a GitHub App or a fine-grained token with the right to open pull requests, so that CI runs on them.

The changesets files, scripts and dependencies are removed; the JavaScript packages are not released from here.

## Decisions for Sebastian

1. The channels of the first release: crates.io only (proposed), or npm and binaries as well.
2. One version for all crates (proposed), or each crate its own.
3. The first version: `0.1.0` (proposed).
4. The schema copy in `ferrigura-model` (proposed), or the generated Rust code checked in.
5. The token for the release pull requests: a GitHub App (proposed) or a fine-grained personal token.
6. The trademark check: who does it, and when the first publish may follow.
