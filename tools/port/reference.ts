/**
 * Loads modules of the TypeScript reference (`packages/`) for the port exporters.
 *
 * The reference imports Vite-only forms such as `config.schema.yaml?only-defaults=true`, which
 * Node cannot load, so the modules go through Vite's SSR loader with the schema plugin the
 * tests use. The exporters themselves run under tsx.
 *
 * The loader's root is the reference's checkout (`reference-root.ts`), and the Jison and schema
 * plugins are that checkout's own, so another checkout is loaded the way its tests load it.
 */
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

import { JSDOM } from 'jsdom';
import { createServer, type Plugin } from 'vite';

import { referenceRoot } from './reference-root.ts';

/** One of the reference checkout's Vite plugins, `.vite/<file>`. */
async function referencePlugin(file: string): Promise<Plugin> {
  const url = pathToFileURL(join(referenceRoot, '.vite', file)).href;
  const module = (await import(url)) as { default: () => Plugin };
  return module.default();
}

export interface Reference {
  /** Loads a module by its path relative to the reference's root, with a leading slash. */
  load<T>(path: string): Promise<T>;
  close(): Promise<void>;
}

/**
 * Opens the reference's module loader. `extraPlugins` lets one exporter add a transform of its
 * own, such as a read of a module's state that the module does not export (`export-kanban.ts`);
 * the reference's source is not changed.
 */
export async function openReference(extraPlugins: Plugin[] = []): Promise<Reference> {
  const server = await createServer({
    configFile: false,
    root: referenceRoot,
    logLevel: 'warn',
    appType: 'custom',
    plugins: [
      await referencePlugin('jsonSchemaPlugin.ts'),
      await referencePlugin('jisonPlugin.ts'),
      ...extraPlugins,
    ],
    resolve: { extensions: ['.ts', '.js'] },
    server: { middlewareMode: true, hmr: false, watch: null },
    optimizeDeps: { noDiscovery: true, include: [] },
  });
  return {
    load: <T>(path: string) => server.ssrLoadModule(path) as Promise<T>,
    close: () => server.close(),
  };
}

/**
 * Gives the reference's DOMPurify a window. DOMPurify takes its window when its module loads, so
 * this runs before the reference's modules load, and it is needed wherever `sanitizeText` runs
 * (the flowchart DB sanitises node, edge and title texts). `check-header.ts` does the same.
 */
export function provideDomForSanitising(): void {
  if ('window' in globalThis) return;
  const dom = new JSDOM('<!doctype html><html><body></body></html>');
  Object.assign(globalThis, { window: dom.window, document: dom.window.document });
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
 * Registers the detectors of the diagram types the port covers: the P1 types, and the P2 and P3
 * types whose parsers are not ported yet. The reference's directive handling calls `detectType`, which
 * throws unless the detectors of the diagrams are registered, as `mermaidAPI` does when it loads
 * the diagrams.
 */
export async function registerDetectors(reference: Reference): Promise<void> {
  const [
    detectType,
    pie,
    info,
    flowchart,
    timeline,
    journey,
    gitGraph,
    classPlugin,
    sequence,
    state,
    gantt,
  ] = await Promise.all([
    reference.load<{ registerLazyLoadedDiagrams(...diagrams: unknown[]): void }>(
      '/packages/mermaid/src/diagram-api/detectType.ts'
    ),
    reference.load<{ pie: unknown }>('/packages/mermaid/src/diagrams/pie/pieDetector.ts'),
    reference.load<{ info: unknown }>('/packages/mermaid/src/diagrams/info/infoDetector.ts'),
    reference.load<{ default: unknown }>(
      '/packages/mermaid/src/diagrams/flowchart/flowDetector-v2.ts'
    ),
    reference.load<{ default: unknown }>('/packages/mermaid/src/diagrams/timeline/detector.ts'),
    reference.load<{ default: unknown }>(
      '/packages/mermaid/src/diagrams/user-journey/journeyDetector.ts'
    ),
    reference.load<{ default: unknown }>('/packages/mermaid/src/diagrams/git/gitGraphDetector.ts'),
    reference.load<{ default: unknown }>(
      '/packages/mermaid/src/diagrams/class/classDetector-V2.ts'
    ),
    reference.load<{ default: unknown }>(
      '/packages/mermaid/src/diagrams/sequence/sequenceDetector.ts'
    ),
    reference.load<{ default: unknown }>(
      '/packages/mermaid/src/diagrams/state/stateDetector-V2.ts'
    ),
    reference.load<{ default: unknown }>('/packages/mermaid/src/diagrams/gantt/ganttDetector.ts'),
  ]);
  detectType.registerLazyLoadedDiagrams(
    pie.pie,
    info.info,
    flowchart.default,
    timeline.default,
    journey.default,
    gitGraph.default,
    classPlugin.default,
    sequence.default,
    state.default,
    gantt.default
  );

  // The P2 detectors, in the order the reference registers them (`diagram-orchestration.ts`). No
  // two of these types match the same text, so the order decides nothing between them.
  const [
    mindmap,
    architecture,
    c4,
    kanban,
    requirement,
    quadrantChart,
    sankey,
    packet,
    xychart,
    block,
    eventmodeling,
    ishikawa,
    treemap,
  ] = await Promise.all([
    reference.load<{ default: unknown }>('/packages/mermaid/src/diagrams/mindmap/detector.ts'),
    reference.load<{ default: unknown }>(
      '/packages/mermaid/src/diagrams/architecture/architectureDetector.ts'
    ),
    reference.load<{ default: unknown }>('/packages/mermaid/src/diagrams/c4/c4Detector.ts'),
    reference.load<{ default: unknown }>('/packages/mermaid/src/diagrams/kanban/detector.ts'),
    reference.load<{ default: unknown }>(
      '/packages/mermaid/src/diagrams/requirement/requirementDetector.ts'
    ),
    reference.load<{ default: unknown }>(
      '/packages/mermaid/src/diagrams/quadrant-chart/quadrantDetector.ts'
    ),
    reference.load<{ default: unknown }>('/packages/mermaid/src/diagrams/sankey/sankeyDetector.ts'),
    reference.load<{ packet: unknown }>('/packages/mermaid/src/diagrams/packet/detector.ts'),
    reference.load<{ default: unknown }>(
      '/packages/mermaid/src/diagrams/xychart/xychartDetector.ts'
    ),
    reference.load<{ default: unknown }>('/packages/mermaid/src/diagrams/block/blockDetector.ts'),
    reference.load<{ default: unknown }>(
      '/packages/mermaid/src/diagrams/eventmodeling/detector.ts'
    ),
    reference.load<{ ishikawa: unknown }>(
      '/packages/mermaid/src/diagrams/ishikawa/ishikawaDetector.ts'
    ),
    reference.load<{ treemap: unknown }>('/packages/mermaid/src/diagrams/treemap/detector.ts'),
  ]);
  detectType.registerLazyLoadedDiagrams(
    mindmap.default,
    architecture.default,
    c4.default,
    kanban.default,
    requirement.default,
    quadrantChart.default,
    sankey.default,
    packet.packet,
    xychart.default,
    block.default,
    eventmodeling.default,
    ishikawa.ishikawa,
    treemap.treemap
  );

  // The P3 detectors, in the order the reference registers them (`diagram-orchestration.ts`). No
  // two of these detectors match the same text, so the order decides nothing between them.
  const [
    agentflow,
    swimlanes,
    treeView,
    radar,
    railroad,
    railroadEbnf,
    railroadAbnf,
    railroadPeg,
    venn,
    wardley,
    cynefin,
    usecase,
  ] = await Promise.all([
    reference.load<{ default: unknown }>('/packages/mermaid/src/diagrams/agentflow/afDetector.ts'),
    reference.load<{ default: unknown }>('/packages/mermaid/src/diagrams/swimlanes/detector.ts'),
    reference.load<{ default: unknown }>('/packages/mermaid/src/diagrams/treeView/detector.ts'),
    reference.load<{ radar: unknown }>('/packages/mermaid/src/diagrams/radar/detector.ts'),
    reference.load<{ railroad: unknown }>(
      '/packages/mermaid/src/diagrams/railroad/railroadDetector.ts'
    ),
    reference.load<{ railroadEbnf: unknown }>(
      '/packages/mermaid/src/diagrams/railroad/ebnfDetector.ts'
    ),
    reference.load<{ railroadAbnf: unknown }>(
      '/packages/mermaid/src/diagrams/railroad/abnfDetector.ts'
    ),
    reference.load<{ railroadPeg: unknown }>(
      '/packages/mermaid/src/diagrams/railroad/pegDetector.ts'
    ),
    reference.load<{ default: unknown }>('/packages/mermaid/src/diagrams/venn/vennDetector.ts'),
    reference.load<{ default: unknown }>(
      '/packages/mermaid/src/diagrams/wardley/wardleyDetector.ts'
    ),
    reference.load<{ cynefin: unknown }>(
      '/packages/mermaid/src/diagrams/cynefin/cynefinDetector.ts'
    ),
    reference.load<{ usecase: unknown }>(
      '/packages/mermaid/src/diagrams/usecase/usecaseDetector.ts'
    ),
  ]);
  detectType.registerLazyLoadedDiagrams(
    agentflow.default,
    swimlanes.default,
    treeView.default,
    radar.radar,
    railroad.railroad,
    railroadEbnf.railroadEbnf,
    railroadAbnf.railroadAbnf,
    railroadPeg.railroadPeg,
    venn.default,
    wardley.default,
    cynefin.cynefin,
    usecase.usecase
  );
}

/**
 * Registers the ER detector, which `registerDetectors` leaves out: the exporters of ER and the
 * configuration run need it, since a directive or a configuration scope detects the diagram type.
 */
export async function registerErDetector(reference: Reference): Promise<void> {
  const [detectType, er] = await Promise.all([
    reference.load<{ registerLazyLoadedDiagrams(...diagrams: unknown[]): void }>(
      '/packages/mermaid/src/diagram-api/detectType.ts'
    ),
    reference.load<{ default: unknown }>('/packages/mermaid/src/diagrams/er/erDetector.ts'),
  ]);
  detectType.registerLazyLoadedDiagrams(er.default);
}
