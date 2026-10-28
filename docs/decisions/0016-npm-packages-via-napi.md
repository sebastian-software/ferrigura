# 0016 npm packages via napi

**Scope:** how ferrigura reaches Node.js: the addon crate `ferrigura-node`, the npm packages under `node/`, and their build and release. It does not cover browsers or WebAssembly, which ferrigura does not target ([0006](./0006-rust-only-no-browser.md)).

## Decision

Node.js gets a native addon built with napi-rs from `crates/ferrigura-node`, which stays off crates.io.

- `@ferrigura/core` (`node/ferrigura`) is the facade: the JavaScript API, its types and a loader that picks the platform's binary.
- Seven optional packages `@ferrigura/<platform>` (`node/platforms/`) carry one binary each: macOS on arm64, Linux on x64 and arm64 with glibc and with musl, and Windows on x64 and arm64.
- The build, the loader, the package checks and the release are the shared lifecycle of `ferramenta-native`, a development dependency pinned to a commit; `node/native-npm.config.mjs` declares the paths, the size budgets, the smoke test and the CI jobs a release waits for.
- A release builds every target from the release's commit after its CI passed, publishes the platform packages before the facade, and checks the published versions and their provenance ([0004](./0004-one-version-one-release.md)).

## Why

An addon is called in-process, with no process per call. One facade with optional platform packages installs only the binary the machine needs, as other native npm packages do.

## Consequences

- The npm packages share the product version and are released with the crates; their switch is the repository variable `NPM_PUBLISH_ENABLED`.
- Trusted Publishing cannot create a package, so the `@ferrigura` scope and each package are bootstrapped once by hand, each with its Trusted Publisher for `publish.yml` and the environment `npm-release`.
- Each platform package carries the notices of its binary ([0013](./0013-licenses-of-ported-code.md)).

## Related

[0004](./0004-one-version-one-release.md), [0006](./0006-rust-only-no-browser.md), [0013](./0013-licenses-of-ported-code.md).
