/**
 * Loads modules of the TypeScript reference (`packages/`) for the port exporters.
 *
 * The reference imports Vite-only forms such as `config.schema.yaml?only-defaults=true`, which
 * Node cannot load, so the modules go through Vite's SSR loader with the schema plugin the
 * tests use. The exporters themselves run under tsx.
 */
import { fileURLToPath } from 'node:url';

import { createServer } from 'vite';

import jsonSchemaPlugin from '../../.vite/jsonSchemaPlugin.ts';

const root = fileURLToPath(new URL('../../', import.meta.url));

export interface Reference {
  /** Loads a module by its path relative to the repository root, with a leading slash. */
  load<T>(path: string): Promise<T>;
  close(): Promise<void>;
}

export async function openReference(): Promise<Reference> {
  const server = await createServer({
    configFile: false,
    root,
    logLevel: 'warn',
    appType: 'custom',
    plugins: [jsonSchemaPlugin()],
    resolve: { extensions: ['.ts', '.js'] },
    server: { middlewareMode: true, hmr: false, watch: null },
    optimizeDeps: { noDiscovery: true, include: [] },
  });
  return {
    load: <T>(path: string) => server.ssrLoadModule(path) as Promise<T>,
    close: () => server.close(),
  };
}

/** One input of the differential run. `id` ties the output back to the input. */
export interface PortInput {
  id: number;
  source: string;
  text: string;
}

/** One output of an exporter: the canonical model, or the reason the reference rejected the input. */
export type PortOutput =
  | { id: number; ok: true; model: unknown }
  | { id: number; ok: false; error: string };

/**
 * Registers the detectors of the diagram types the port covers. The reference's directive
 * handling calls `detectType`, which throws unless the detectors of the diagrams are registered,
 * as `mermaidAPI` does when it loads the diagrams.
 */
export async function registerDetectors(reference: Reference): Promise<void> {
  const [detectType, pie, info] = await Promise.all([
    reference.load<{ registerLazyLoadedDiagrams(...diagrams: unknown[]): void }>(
      '/packages/mermaid/src/diagram-api/detectType.ts'
    ),
    reference.load<{ pie: unknown }>('/packages/mermaid/src/diagrams/pie/pieDetector.ts'),
    reference.load<{ info: unknown }>('/packages/mermaid/src/diagrams/info/infoDetector.ts'),
  ]);
  detectType.registerLazyLoadedDiagrams(pie.pie, info.info);
}
