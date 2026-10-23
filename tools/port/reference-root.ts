/**
 * The checkout the harness takes the reference from: upstream Mermaid at the pin, with its patches,
 * as `tools/reference/setup.sh` builds it in `target/reference/<commit>` (docs-port/ORACLE-PLAN.md).
 * `REFERENCE_ROOT`, an absolute path, names another checkout instead. The reference's `packages/`
 * (sources, specs, docs and the built bundle), its `node_modules` and its `.vite` plugins are used;
 * the corpus (`e2e/diagrams`, `e2e/platform/dev-diagrams`, the cases under `tools/port`) and
 * ferrigura always come from this repository.
 */
import { existsSync, readFileSync } from 'node:fs';
import { join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

/** This repository's root, with a trailing separator. */
export const repositoryRoot = fileURLToPath(new URL('../../', import.meta.url));

/** The commit of `tools/reference/pin`. */
function pinnedCommit(): string {
  const pin = readFileSync(join(repositoryRoot, 'tools/reference/pin'), 'utf8');
  const commit = /^commit (\S+)$/m.exec(pin)?.[1];
  if (commit === undefined) throw new Error('tools/reference/pin names no commit');
  return commit;
}

/** The reference's root, with a trailing separator. */
export const referenceRoot = process.env.REFERENCE_ROOT
  ? resolve(process.env.REFERENCE_ROOT) + sep
  : join(repositoryRoot, 'target/reference', pinnedCommit()) + sep;

/** Stops with what to do when the reference is not built. */
export function requireReference(): void {
  if (!existsSync(join(referenceRoot, 'packages/mermaid/dist'))) {
    throw new Error(
      `the reference is not built in ${referenceRoot}: run tools/reference/setup.sh, or set ` +
        'REFERENCE_ROOT="$(tools/reference/setup.sh)"'
    );
  }
}
