# The first release of ferrigura

The plan for publishing ferrigura for the first time: what goes out, what has to be done before, how releases are made from then on, and the decisions that are Sebastian's. Status: prepared on 2026-10-20, with the decisions below; the steps that do not wait for the trademark check are done. Nothing is published.

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

Done on 2026-10-20, without the trademark check:

| Step                                  | State                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Build inputs inside the crates**    | `ferrigura-model` reads its own copy of the configuration schema, `schema/config.schema.yaml`; the test `schema_copy` fails when the copy and `packages/mermaid/src/schemas/config.schema.yaml` differ. Every other crate reads only its own files (checked with `grep` for `../` in the build scripts and the `include_*!` macros).                                                                                                                                                                                                         |
| **Package contents**                  | Each publishable crate has an `include` list: `src`, its README, licence and notice files, and the model's build script and schema, the text crate's fonts. The packages, compressed: `ferrigura-text` 811 KB (the fonts), `ferrigura-render` 750 KB, `ferrigura-syntax` 476 KB, `ferrigura-elk` 299 KB, `ferrigura-model` 128 KB, `ferrigura-swimlane` 127 KB, `ferrigura-cose` 66 KB, `ferrigura-jsmath` 32 KB, `ferrigura-venn` 26 KB, `ferrigura` 19 KB, `ferrigura-path` 16 KB, `ferrigura-cli` 13 KB; all far below crates.io's 10 MB. |
| **Versions on the path dependencies** | Every dependency between the crates has `version = "0.0.0"` beside its path; release-please raises them with the crates.                                                                                                                                                                                                                                                                                                                                                                                                                     |
| **Metadata**                          | Each crate has `readme`; `ferrigura` and `ferrigura-cli` have `keywords`, `categories` and `documentation`.                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| **Licences in the packages**          | The seven MIT crates without a licence file have the repository's `LICENSE`; the others keep their own (ELK's EPL-2.0, the notices of `ferrigura-jsmath` and `ferrigura-path`). `ferrigura-text` is `MIT AND OFL-1.1 AND CC-BY-4.0`, with the faces' licences and Twemoji's attribution in `fonts/` and `NOTICE.md`.                                                                                                                                                                                                                         |
| **Well-formed XML**                   | Merged on 2026-10-20: every SVG is well-formed XML, and a test parses every fixture's output.                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| **Browsers**                          | Safari 27 draws the colour emoji (spike `fallback-faces.md`, section 8).                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| **Docs**                              | `cargo doc` without warnings, checked in CI with `RUSTDOCFLAGS="-D warnings"`.                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| **Dry run**                           | `cargo package --workspace --exclude ferrigura-node` packages the twelve crates and builds each from its package, in the order of their dependencies (with `publish = true` set for the run only).                                                                                                                                                                                                                                                                                                                                           |

Left for the first release:

1. **Trademark check** for `ferrigura` (DPMA, EUIPO, USPTO), as `STATUS.md` has it. A crate name on crates.io cannot be given back, so this comes first.
2. **The GitHub App** for the release pull requests: Sebastian creates it with the rights to write contents and pull requests on this repository, and sets its client id as the variable `RELEASE_APP_CLIENT_ID` and its private key as the secret `RELEASE_APP_PRIVATE_KEY`.
3. **The crates.io token** for the first publish, as the secret `CARGO_REGISTRY_TOKEN` of the environment `crates-io`; after the first release, crates.io's trusted publishing replaces it.
4. **The two latches**: `publish = false` in `[workspace.package]` becomes `true` (`ferrigura-node` keeps `false`), and the repository variable `PUBLISH_CRATES` is set to `true`. Until then nothing can be published, by the workflow or by hand.
5. **Firefox**: not checked yet for the colour emoji.

## How releases are made

[release-please](https://github.com/googleapis/release-please) replaces the changesets setup inherited from Mermaid:

- `release-please-config.json` lists the twelve crates with the `rust` release type, the `cargo-workspace` plugin (it updates the versions of the dependencies between the crates) and the `linked-versions` plugin (one version for all). `.release-please-manifest.json` holds the current version of each crate.
- The commit messages are the release notes: `feat(…)` and `fix(…)` as the repository writes them already. Each crate gets a `CHANGELOG.md`.
- `bootstrap-sha` is the commit release-please starts from, so the first changelog does not list the whole history of the port; the first release is forced to `0.1.0` with a `Release-As: 0.1.0` footer when it is due.
- `.github/workflows/release-please.yml` runs release-please. Until the first release it runs only by hand (`workflow_dispatch`), so it opens no release pull request before the steps above are done; then it runs on every push to `main`.
- When a release pull request is merged, release-please tags the commit and makes a GitHub release, and the job `publish` runs `cargo publish --workspace --exclude ferrigura-node`, which uploads the crates in the order of their dependencies. It runs only when the variable `PUBLISH_CRATES` is `true`. crates.io's trusted publishing (OpenID Connect from GitHub Actions) needs no long-lived token, but it can be set up only for a crate that exists: the first version is published with a token of Sebastian's, then trusted publishing takes over.
- A pull request opened with the default `GITHUB_TOKEN` does not start the CI workflow, so release-please opens the release pull requests with a GitHub App's token (`actions/create-github-app-token`).

The changesets files, scripts and dependencies are removed; the JavaScript packages are not released from here.

## Decisions

Decided by Sebastian on 2026-10-20, as proposed:

1. The first release goes to crates.io only; npm and prebuilt binaries come later.
2. One version for all crates.
3. The first version is `0.1.0`.
4. `ferrigura-model` holds a copy of the schema, checked against the source.
5. A GitHub App's token opens the release pull requests.

Open: the trademark check, who does it, and when the first publish may follow.
