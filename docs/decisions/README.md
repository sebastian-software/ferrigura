# Decisions

Each record holds one decision that shapes ferrigura: what it covers, what it decides, why, and what follows from it. The records are living documents. When a decision changes, its record is edited in place; when a decision no longer holds, its record is deleted and its number is not reused. There are no dates, statuses or change logs in a record: what it says is what holds today.

A record has this outline:

- **Scope:** what the decision covers and, as explicitly, what it does not cover.
- **Decision:** the rule, worded so that it can be checked where possible.
- **Why:** the reasons, and the options that were weighed and not chosen.
- **Consequences:** what follows for the code, the tests and the users.
- **Related:** other records and the [learnings](../LEARNINGS.md) behind it.

A new decision that is hard to reverse gets the next free number. Code comments cite a record by its number, such as `decision 0009`.

## Project

| Record                                    | Decision                                                                                     |
| ----------------------------------------- | -------------------------------------------------------------------------------------------- |
| [0001](./0001-us-english.md)              | Code, documentation and commit messages are written in US English                            |
| [0002](./0002-glossary.md)                | The domain terms and the words to avoid                                                      |
| [0003](./0003-conventional-commits.md)    | Commit messages follow Conventional Commits; `feat` and `fix` are for what the packages show |
| [0004](./0004-one-version-one-release.md) | All crates and npm packages share one version, one changelog and one tag                     |
| [0005](./0005-quality-gates.md)           | Every change keeps the whole suite green, and output changes are deliberate                  |

## Architecture

| Record                                            | Decision                                                                                            |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| [0006](./0006-rust-only-no-browser.md)            | Rust only, with no browser at run time                                                              |
| [0007](./0007-hand-written-parsers.md)            | Hand-written parsers; Mermaid's grammars are the specification                                      |
| [0008](./0008-canonical-model-and-typed-views.md) | A canonical model per diagram type; the configuration is resolved once and read through typed views |
| [0009](./0009-svg-text-labels.md)                 | Labels are SVG text, not HTML in `foreignObject`                                                    |
| [0010](./0010-text-measurement-and-fonts.md)      | Only text is measured, with bundled fonts embedded as subsets                                       |
| [0011](./0011-themes-as-css-variables.md)         | Themes are CSS custom properties with attribute fallback                                            |
| [0012](./0012-layout-engines-ported-exactly.md)   | Layout engines and browser arithmetic are ported to give the same numbers                           |
| [0013](./0013-licenses-of-ported-code.md)         | Ported code keeps its source's license, in a crate of its own when it differs                       |
| [0018](./0018-deterministic-output.md)            | The same input gives the same output: no randomness, screen or state from the environment           |

## Verification

| Record                                        | Decision                                                                                                    |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| [0014](./0014-upstream-reference-at-a-pin.md) | The reference is upstream Mermaid at a pinned commit, with patches                                          |
| [0015](./0015-differential-testing.md)        | Models, layouts and geometry are compared with the reference; ferrigura's own output is pinned by snapshots |
| [0017](./0017-reproduce-the-reference.md)     | Quirks of the reference are reproduced; a bug is fixed only together with a patch on the reference          |

## Distribution

| Record                                  | Decision                                                                            |
| --------------------------------------- | ----------------------------------------------------------------------------------- |
| [0016](./0016-npm-packages-via-napi.md) | Node.js gets a native addon built with napi-rs, in one facade and platform packages |
