# 0001 US English

**Scope:** the language of everything written in this repository: documentation, code comments, identifiers, test names, commit messages, pull requests and issues. It does not cover the text of the corpus (`e2e/`), the test inputs taken from Mermaid, the names that come from Mermaid's syntax or configuration (`colorScheme`, `themeVariables`), or third-party license and notice texts, which stay as their sources write them.

## Decision

The project language is English with US spelling: _color_, _center_, _behavior_, _license_ (the noun too), _sanitize_, _serialize_, _normalize_.

## Why

One language and one spelling make the code searchable: a reader who greps for `color` finds every place, not half of them. Mermaid, the configuration keys ferrigura reads and the Rust and npm ecosystems use US spelling, so the project reads as one with them.

## Consequences

- Documentation, comments and internal names use US spelling. Ported test names keep the wording of Mermaid's spec titles ([0015](./0015-differential-testing.md)).
- Public items of the published crates that still have British spelling (`colour`, `centre` and their compounds) keep it until a release that may break the interface renames them, since renaming one is a breaking change ([0004](./0004-one-version-one-release.md)).
- String literals keep their spelling where they become output or match a name of Mermaid's; elsewhere, a change that touches one converts it.

## Related

[0003](./0003-conventional-commits.md) for commit messages.
