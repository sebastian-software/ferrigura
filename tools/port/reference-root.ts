/**
 * The checkout the harness takes the TypeScript reference from. By default that is this repository,
 * whose `packages/` hold the fork the port is checked against. `REFERENCE_ROOT`, an absolute path,
 * names another checkout instead, such as upstream Mermaid at the commit the fork starts from: its
 * `packages/` (sources, specs, docs and the built bundle), its `node_modules` and its `.vite`
 * plugins are then used. The corpus (`e2e/diagrams`, `e2e/platform/dev-diagrams`, the cases under
 * `tools/port`) and ferrigura always come from this repository.
 */
import { resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

/** This repository's root, with a trailing separator. */
export const repositoryRoot = fileURLToPath(new URL('../../', import.meta.url));

/** The reference's root, with a trailing separator. */
export const referenceRoot = process.env.REFERENCE_ROOT
  ? resolve(process.env.REFERENCE_ROOT) + sep
  : repositoryRoot;
