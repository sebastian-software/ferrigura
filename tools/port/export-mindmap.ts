/**
 * Canonical JSON of mindmap diagrams, from the TypeScript reference.
 *
 * The steps are those of `Diagram.fromText` after the configuration of the input is set: the DB
 * is cleared, the text (with `encodeEntities` and a line break) goes to the Jison parser, which
 * fills the DB through its calls. The model is the tree the DB holds after the parse, from its
 * first node. The DB keeps the entity placeholders and does not decode them, so the texts are
 * written as the DB holds them. The reference's DOMPurify needs a window, as for the flowchart.
 *
 * `getData` and the layout are not read: they are the renderer's (`docs-port/mindmap/PLAN.md`,
 * section 4). The padding and the width are read from the configuration at the time of each
 * `addNode`, as the DB does, and a value the configuration gives as a string, a boolean or an array
 * stays as it was given.
 *
 * Usage: `tsx tools/port/export-mindmap.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

/** A node as the DB stores it after the parse (`MindmapNode` in `mindmapTypes.ts`). */
interface DbNode {
  id: number;
  nodeId: string;
  descr: string;
  type: number;
  level: number;
  width: unknown;
  padding: unknown;
  icon?: string;
  class?: string;
  children: DbNode[];
}

interface CanonicalNode {
  id: number;
  nodeId: string;
  descr: string;
  type: number;
  level: number;
  width: unknown;
  padding: unknown;
  icon: string;
  class: string;
  children: CanonicalNode[];
}

interface CanonicalModel {
  version: number;
  root: CanonicalNode | null;
}

interface Modules {
  config: {
    setSiteConfig(conf: Record<string, unknown>): unknown;
    reset(): void;
    getConfig(): Record<string, unknown>;
    setDiagramConfigScope(diagramType?: string): void;
    addDirective(directive: Record<string, unknown>): void;
  };
  preprocess: {
    preprocessDiagram(code: string): {
      code: { cleaned: string };
      title?: string;
      config: Record<string, unknown>;
    };
  };
  detectType: {
    detectType(text: string, config?: unknown): string;
  };
  encode: { encodeEntities(text: string): string };
  grammar: { yy: unknown; parse(text: string): unknown };
  db: {
    MindmapDB: new () => {
      clear(): void;
      getMindmap(): DbNode | null;
    };
  };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [config, preprocess, detectType, encode, grammar, db] = await Promise.all([
    reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: Modules['grammar'] }>(
      '/packages/mermaid/src/diagrams/mindmap/parser/mindmap.jison'
    ),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/mindmap/mindmapDb.ts'),
  ]);
  return { config, preprocess, detectType, encode, grammar: grammar.parser, db };
}

/**
 * A value the configuration gave, as JSON: a non-finite number is its sentinel string (`value.rs`),
 * inside arrays and objects too, since `JSON.stringify` would write it as `null`.
 */
function jsonValue(value: unknown): unknown {
  if (typeof value === 'number') return Number.isFinite(value) ? value : String(value);
  if (Array.isArray(value)) return value.map(jsonValue);
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, jsonValue(item)]));
  }
  return value;
}

/** One node of the DB, and its children, in the shape of the canonical model. */
function canonicalNode(node: DbNode): CanonicalNode {
  return {
    id: node.id,
    nodeId: node.nodeId,
    descr: node.descr,
    type: node.type,
    level: node.level,
    width: jsonValue(node.width),
    padding: jsonValue(node.padding),
    icon: node.icon ?? '',
    class: node.class ?? '',
    children: node.children.map(canonicalNode),
  };
}

await runExporter(loadModules, (input, { config, preprocess, detectType, encode, grammar, db }) => {
  // The configuration of the input alone, as a render gives it (`processAndSetConfigs`).
  config.setSiteConfig({});
  config.reset();
  const processed = preprocess.preprocessDiagram(input.text);
  let diagramType: string | undefined;
  try {
    diagramType = detectType.detectType(processed.code.cleaned, config.getConfig());
  } catch {
    diagramType = undefined;
  }
  config.setDiagramConfigScope(diagramType);
  config.addDirective(processed.config);

  const mindmapDb = new db.MindmapDB();
  mindmapDb.clear();
  grammar.yy = mindmapDb;
  grammar.parse(encode.encodeEntities(processed.code.cleaned) + '\n');

  const root = mindmapDb.getMindmap();
  return { version: 1, root: root === null ? null : canonicalNode(root) } satisfies CanonicalModel;
});
