# P2 parsers: how a parser task works

Each parser task ports one P2 diagram type's parser and DB, in its own worktree and branch cut from `sebastian/port-p2`. The coordinator reviews the report and merges the branch. The research is in `docs-port/<type>/NOTES.md`; the decisions in [P2-PLAN.md](./P2-PLAN.md) and [M6-PLAN.md](./M6-PLAN.md) apply.

## What is already there

- **The scaffold**: the type is detected (`crates/ferrigura-syntax/src/lib.rs`), has an empty model (`crates/ferrigura-model/src/<module>.rs`), a parser stub (`crates/ferrigura-syntax/src/<module>/` or `<module>.rs`) that reports "not ported yet", an arm in the `canonical` example (`crates/ferrigura-syntax/examples/canonical.rs`), and its entries in `tools/port/` (`inputs.ts`, `reference.ts`, `diff.ts`, `exceptions.ts`, `cases.ts`, `status.ts`, `diff-config.ts`). `pnpm port:diff <type>` reports the type as not ported until `tools/port/export-<type>.ts` exists.
- **Worked examples** of the whole route, from P1: journey (Jison; `crates/ferrigura-syntax/src/journey/`, `tools/port/export-journey.ts`, `docs-port/journey/PLAN.md`), gitGraph and pie (Langium, parsed by hand; `crates/ferrigura-syntax/src/git_graph/`, `pie.rs`), and the shared helpers: `preprocess.rs` (front matter, `removeDirectives`, `cleanupComments`, `encodeEntities`), `sanitize.rs` and `entities.rs` (DOMPurify's subset and the HTML entities), `url.rs` (`sanitizeUrl`), `yaml.rs`, `lexer.rs`. Read `docs-port/decisions/0002-hand-written-parsers.md` and `docs-port/canonical-model.md`.

## What a parser task delivers

1. **The model** in `ferrigura-model`: the DB state after the parse, as `NOTES.md` section 4 proposes, camelCase in JSON, with `version`.
2. **The parser and the DB** in `ferrigura-syntax`: the reference's preprocessing, a hand-written scanner in the generated lexer's rule order (Jison and Chevrotain take the first rule that matches, in their order, not the longest match), the grammar, and the DB calls in their order with their sanitising and defaults. Every quirk of `NOTES.md` sections 2 and 3 is reproduced.
3. **The exporter** `tools/port/export-<type>.ts`: it runs the reference's own parser and DB on every input (as `export-journey.ts` does) and writes the canonical JSON; the `canonical` example's arm writes ferrigura's. `pnpm port:diff <type>` must show no difference on every collected input. Add the inputs the notes probed as explicit cases in `cases.ts`.
4. **The specs**: the reference's parser and DB specs as Rust tests in `crates/ferrigura-syntax/tests/<module>_*.rs`, listed in `status.ts`; a spec that tests the renderer or the layout data stays as an ignored test that names that reason.
5. **The configuration**: the type's group in `pnpm port:diff config` compares its own section once the type is ported (`diff-config.ts`, `UNPORTED_SECTIONS`); the configuration types come from the schema (`crates/ferrigura-model/build.rs`).
6. **The docs**: `docs-port/<type>/PLAN.md` with the parser decisions and every reproduced quirk, and a paragraph in `docs-port/canonical-model.md`.

## Rules that every type shares

- A rejection on both sides counts as agreement; the error text is ferrigura's own, with the position (decision 0002).
- Texts are what the DB holds: sanitised where the DB sanitises, with the entity placeholders where the DB keeps them.
- Ids that the reference draws at random are generated deterministically in the reference's format, and the differential replaces them by placeholders in order on both sides (as gitGraph does, `normalise_auto_ids` in `canonical.rs`).
- A named exception in `tools/port/exceptions.ts` only where the reference's output is garbage (NaN, a crash, a random value); every other difference is fixed.

## Gates before the report

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --no-fail-fast`; `pnpm port:diff <type>` and `pnpm port:diff config`; `pnpm exec prettier --check tools/port` and `pnpm exec eslint tools/port`.

## Rules of the branch

Work only in the worktree you were given; commit there with the trailer `Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>`; do not merge, rebase, push or use `git stash`. Own files: your type's model, parser, tests, exporter and docs. A change to a shared file (`lib.rs` of a crate, `preprocess.rs`, `tools/port/*.ts` beyond your type's entries) is allowed when the task needs it; keep it small and name it in the report. If something blocks you, stop and say so.

## The report

The differential's result (inputs, accepted, differences, named exceptions), the specs (ported, passing, ignored with reasons), the gates, the shared files you changed, the quirks you reproduced, open questions.
