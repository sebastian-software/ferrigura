# 0004 One version, one release

**Scope:** how the published packages are versioned and released: the twelve crates on crates.io and the npm packages `@ferrigura/core` and `@ferrigura/<platform>`. It does not cover the harness under `tools/` or the crate `ferrigura-node`, which are not published.

## Decision

ferrigura is one product with one version.

- Every crate and npm package carries the same version and is released together: a change in any of them is a release of all of them, which is how they are tested.
- release-please makes the releases from the commit messages ([0003](./0003-conventional-commits.md)): one component, `ferrigura` at the repository's root, with the `rust` release type. The root `Cargo.toml` is the package `ferrigura` (its sources are in `crates/ferrigura`), so the Rust strategy reads its version and moves every member of the workspace, the versions of the dependencies between them and `Cargo.lock` with it. The npm manifests follow as extra files (`release-please-config.json`).
- One release pull request, one changelog (`CHANGELOG.md` at the root), one tag `v<version>` and one GitHub release.
- Before `1.0.0`, a feature raises the minor version and a fix the patch version (`bump-minor-pre-major`), as Cargo reads `0.x` versions: `0.4.0` is a breaking change for `0.3.x` users.
- `.github/workflows/publish.yml` publishes a release from its tag. Crates go to crates.io through Trusted Publishing, in the order of their dependencies; npm packages go through Trusted Publishing from the environment `npm-release`, the platform packages before the facade ([0016](./0016-npm-packages-via-napi.md)). Each registry has its own switch, the repository variables `PUBLISH_ENABLED` and `NPM_PUBLISH_ENABLED`.

## Why

The crates are built and tested as one workspace, and the npm packages wrap the same build; versions that drift apart would describe combinations nobody tested. One version also gives users one number to report and one changelog to read.

## Consequences

- Every dependency between the crates has `version` beside its `path`, which release-please raises with the crates.
- Each published crate has an `include` list (its sources, README, license and notice files, and what its build reads), and builds from its own package: no build script or `include_*!` reaches outside the crate.
- release-please opens its pull requests with the organization's `RELEASE_PLEASE_TOKEN`, since a pull request opened with `GITHUB_TOKEN` does not start CI.
- Trusted Publishing cannot create a crate or an npm package. A new crate is published once by hand with `tools/release/publish-crates.sh <tag>`, which publishes the same list in the same order as the workflow; a manual run of `publish.yml` with a release's tag retries a failed publish.

## Related

[0003](./0003-conventional-commits.md), [0013](./0013-licenses-of-ported-code.md), [0016](./0016-npm-packages-via-napi.md).
