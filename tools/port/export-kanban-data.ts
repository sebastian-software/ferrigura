/**
 * Layout data of kanban diagrams from the TypeScript reference: `KanbanDB.getData()` after the text
 * is parsed, with the configuration a render has (`kanbanRenderer.ts`, `draw`).
 *
 * For each input the configuration is set up as `mermaidAPI` sets it before a render: the
 * configuration is reset, the text goes through `preprocessDiagram`, its diagram type is detected,
 * the diagram scope is set (so `look` resolves as the kanban scope gives it) and the directives are
 * added. The parser then fills the DB, as `Diagram.fromText` does (`encodeEntities` and a line
 * break), and `getData()` is read out as `{ nodes }`, with the names and the shape of the Rust data
 * (`crates/ferrigura-render/src/kanban/data.rs`):
 *
 * - a field the reference leaves `undefined` is absent, as JSON writes it; the Rust side writes
 *   only the fields it holds;
 * - a non-finite number is the string `"NaN"`, `"Infinity"` or `"-Infinity"`, as `value.rs` writes it;
 * - the configuration itself is not written, only the nodes.
 *
 * Usage: `tsx tools/port/export-kanban-data.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

type Json = Record<string, unknown>;

interface KanbanDataNode {
  id: string;
  label: string;
  labelType: string;
  isGroup: boolean;
  ticket?: string;
  parentId?: string;
  priority?: unknown;
  assigned?: string;
  icon?: string;
  shape: string;
  level: number;
  look?: string;
  rx?: number;
  ry?: number;
  cssStyles?: string[];
}

interface KanbanDb {
  clear(): void;
  getData(): { nodes: KanbanDataNode[] };
}

interface Modules {
  config: {
    setSiteConfig(conf: Json): unknown;
    reset(): void;
    getConfig(): Json;
    setDiagramConfigScope(diagramType?: string): void;
    addDirective(directive: Json): void;
  };
  detectType: {
    detectType(text: string, config?: Json): string;
  };
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; config?: Json };
  };
  utils: {
    encodeEntities(text: string): string;
  };
  grammar: { yy: unknown; parse(text: string): unknown };
  db: { default: KanbanDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [config, detectType, preprocess, utils, grammar, db] = await Promise.all([
    reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: Modules['grammar'] }>(
      '/packages/mermaid/src/diagrams/kanban/parser/kanban.jison'
    ),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/kanban/kanbanDb.ts'),
  ]);
  return { config, detectType, preprocess, utils, grammar: grammar.parser, db };
}

/** A value as JSON: non-finite numbers become the strings the configuration exporter uses. */
function json(value: unknown): unknown {
  if (typeof value === 'number') {
    if (Number.isFinite(value)) return value;
    return Number.isNaN(value) ? 'NaN' : value > 0 ? 'Infinity' : '-Infinity';
  }
  if (Array.isArray(value)) return value.map(json);
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, json(item)]));
  }
  return value;
}

/** A node as the Rust data writes it: the reference's fields, with `undefined` left out. */
function nodeModel(node: KanbanDataNode): Json {
  const model: Json = {};
  for (const [key, value] of Object.entries(node)) {
    if (value === undefined) continue;
    model[key] = json(value);
  }
  return model;
}

/** The layout data of the text, after its configuration and its parse. */
function layoutOf(text: string, modules: Modules): Json {
  const { config, detectType, preprocess, utils, grammar, db } = modules;

  config.setSiteConfig({});
  config.reset();
  const processed = preprocess.preprocessDiagram(text);
  let diagramType: string | undefined;
  try {
    diagramType = detectType.detectType(processed.code.cleaned, config.getConfig());
  } catch {
    diagramType = undefined;
  }
  config.setDiagramConfigScope(diagramType);
  config.addDirective(processed.config ?? {});
  try {
    const kanbanDb = db.default;
    kanbanDb.clear();
    grammar.yy = kanbanDb;
    grammar.parse(utils.encodeEntities(processed.code.cleaned) + '\n');
    return { nodes: kanbanDb.getData().nodes.map(nodeModel) };
  } finally {
    config.setDiagramConfigScope(undefined);
  }
}

await runExporter(loadModules, (input, modules) => layoutOf(input.text, modules));
