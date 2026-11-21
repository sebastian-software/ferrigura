# 0022 The stable public API

**Scope:** the compatibility promise for Ferrigura 1.x consumers of the Rust facade, the CLI, and the npm package.

## Decision

- The Rust promise covers the documented public API of `ferrigura`, including rendering, configuration, font registration, and selection of the built-in diagram plugins. Types and traits exposed by that API carry the same promise when used through the facade. The harness entry points and seed fields marked `#[doc(hidden)]` are internal.
- The npm promise covers the documented `@ferrigura/core` exports, TypeScript declarations, and rendering behavior. The CLI promise covers its documented arguments, output, and exit behavior.
- The separately published implementation crates provide experimental APIs for direct consumers. Their workspace version identifies the build they ship with ([0004](./0004-one-version-one-release.md)). Direct consumers can pin an exact version. Each released facade must continue to build with every internal dependency version allowed by its manifest. A change incompatible with an older facade requires dependency constraints that keep those releases buildable.
- Supported configuration keys, value meanings, and precedence are part of the promise. The default uses adaptive flowchart geometry and compact sequence participants. The explicit Mermaid-compatible mode remains available ([0019](./0019-adaptive-flowchart-mode.md)).
- The documented Rust error variants and payload types are stable. They cover parse, configuration, options, text, layout, and unsupported diagram errors. In Node.js, rendering errors are thrown. Diagnostic wording and reported source locations may improve within a major version.
- Rendering fixes and reference upgrades may change visual details and SVG bytes while preserving documented behavior. Repeated output under fixed render conditions follows [0018](./0018-deterministic-output.md). Reference upgrades follow the comparison and exception rules in [0014](./0014-upstream-reference-at-a-pin.md).

## Why

Consumers integrating the engine need a predictable rendering interface, including the ability to select diagram plugins. The lower crates expose the parser models, text machinery, and ported layout stages directly, so their interfaces evolve with the implementation and reference. A compatibility promise anchored to the facade keeps that work possible and gives consumers a single place to find supported APIs.

## Consequences

- Adding a required argument, adding a field to a stable struct that consumers can construct exhaustively, or adding a variant to a stable exhaustive enum is a breaking change. New stable APIs that need to grow use construction methods or support nonexhaustive matching from the start.
- Published examples use documented facade paths, including for values and types defined in the implementation crates. Rust compatibility checks compile consumer code against the facade. TypeScript compatibility checks compile against the installed npm package. Output changes continue to run through the snapshot and reference checks in [0005](./0005-quality-gates.md).

## Related

[0004](./0004-one-version-one-release.md), [0018](./0018-deterministic-output.md), [0019](./0019-adaptive-flowchart-mode.md), [0020](./0020-diagram-types-as-plugins.md).
