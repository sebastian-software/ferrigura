# 0003 Conventional Commits

**Scope:** the messages of every commit that lands on `main`, and the titles of pull requests, which become commit messages when they are squashed. It does not cover commits on a branch that are squashed away before they land.

## Decision

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/): `type(scope): summary`, with a body that says why when the summary does not.

- `feat` and `fix` are only for changes that the published packages show: the crates' API or output, the command-line tool, the npm packages. release-please turns exactly these into a release and its changelog ([0004](./0004-one-version-one-release.md)).
- Everything else takes the type that names it: `docs`, `test`, `refactor` (no change in behavior), `perf`, `ci`, `build`, and `chore` for the harness, the tooling and dependency updates of the harness and of CI.
- A breaking change is marked with `!` after the type or a `BREAKING CHANGE:` footer.
- The scope is the diagram type (`flowchart`, `gantt`), the crate without its prefix (`elk`, `text`), or the area (`node`, `harness`, `reference`, `deps`).

## Why

The commit messages are the release notes. A harness or CI change typed `fix` would publish a release of the crates that changes nothing for their users, and a crate change typed `chore` would be missing from the changelog.

## Consequences

- Renovate types the updates of the harness, of CI and of the pin `chore`; updates of the crates' own dependencies stay `fix(deps)`, since they ship (`renovate.json`).
- A reviewer checks the type as part of the change.

## Related

[0001](./0001-us-english.md), [0004](./0004-one-version-one-release.md).
