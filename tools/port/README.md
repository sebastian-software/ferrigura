# Port tooling

Scripts that compare ferrigura with the TypeScript reference. The conventions the canonical models follow are in [`docs-port/canonical-model.md`](../../docs-port/canonical-model.md); the decision to test this way is in [0003](../../docs-port/decisions/0003-differential-testing.md).

| Command                  | What it does                                                                                                     |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| `pnpm port:diff pie`     | Collects the inputs, runs the exporter and `ferrigura-syntax`'s `canonical` example, and reports each difference |
| `pnpm port:diff info`    | The same for info                                                                                                |
| `pnpm port:diff config`  | Runs the effective configuration of the pie inputs and of `config-cases.ts` through both sides, and the key set  |
| `pnpm port:check-header` | Compares the pie header rules in `pie-header.ts` with the real pie DB setters, with jsdom as the DOM             |
| `pnpm port:status`       | Counts the ported Rust specs per type and rewrites the P1 table of `docs-port/STATUS.md`                         |

## Files

- `diff.ts` runs both sides for one type and writes the outputs and a summary to `target/port/`. A rejection on both sides counts as agreement.
- `inputs.ts` collects the inputs: the e2e fixtures, the string literals of the spec files that start like the diagram, the mermaid blocks of the docs, and the explicit cases of `cases.ts`. Identical texts are kept once.
- `cases.ts` holds explicit inputs the reference's tests do not cover, such as title precedence and blank handling in `accDescr` blocks.
- `export-pie.ts` and `export-info.ts` write the canonical JSON of the TypeScript reference. They run with tsx, and `exporter.ts` holds their shared loop.
- `export-config.ts` runs the steps of `mermaidAPI` for each input (site settings, reset, `preprocessDiagram`, the diagram scope, `addDirective`) and writes `getConfig()` and `configKeys`. `diff-config.ts` compares them with `cargo run --example config`; `config-cases.ts` holds the explicit inputs. The comparison covers the top level and the pie section: other diagram sections are compared when their type is ported, because the reference fills them with its own generator (`.build/jsonSchema.ts`).
- `pie-header.ts` holds the pie header rules: title precedence and the two DB whitespace rules. Both the exporter and the check use it.
- `check-header.ts` runs the real pie DB setters and compares them with `pie-header.ts`, on the inputs without `&`, `<`, `>` or no-break spaces.
- `reference.ts` loads modules of the reference and registers the detectors.
- `status.ts` writes the status table.

## Why the reference loads through Vite SSR

The exporters run with `tsx`, but they load the reference's modules through Vite's SSR loader (`createServer` with `ssrLoadModule`), not through Node's module loader. The reason: the reference's configuration imports `packages/mermaid/src/schemas/config.schema.yaml?only-defaults=true`. The `?only-defaults` query is handled by the schema plugin in `.vite/`, which the tests also use, and Node and tsx cannot load a YAML file with that query. Vite runs the same plugins as the tests, so the reference is loaded the way it is tested.

Two details follow from that:

- Detectors must be registered before `preprocessDiagram` runs. The reference's directive handling calls `detectType`, which throws for any directive if no detector is registered. `registerDetectors` in `reference.ts` does what `mermaidAPI` does when it loads the diagrams.
- jsdom is used only by `check-header.ts`. DOMPurify, which the DB setters call, needs a window when it is loaded, so the check sets one first. The exporters never touch the DB setters for header texts, so they run without a DOM.
