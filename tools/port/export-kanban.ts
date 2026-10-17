/**
 * Canonical JSON of kanban diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram`, then `encodeEntities` and a line break, as
 * `Diagram.fromText` gives it to the Jison parser. The parser calls the DB in document order, and
 * the DB's state is the model: every node, in the order the DB holds it. The DB has no accessor for
 * its node list, and `getData` drops the section fields and sanitises the labels a second time, so
 * the exporter reads the list through `exposeNodes`, a line the loader appends to `kanbanDb.ts`
 * (`openReference`'s extra plugins). The reference's source is not changed.
 *
 * The DB's texts are what it holds: the placeholders of the entities stay. A number, a boolean, an
 * array or an object of a label or priority is written as JSON, and a non-finite number as the
 * string `"NaN"`, `"Infinity"` or `"-Infinity"`, as `value.rs` writes it.
 *
 * Usage: `tsx tools/port/export-kanban.ts inputs.json outputs.json`
 */
import type { Plugin } from 'vite';

import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

/** A node of the DB object, with the fields `addNode` and `decorateNode` set. */
interface KanbanDbNode {
  id: string;
  level: number;
  label: unknown;
  parentId?: string;
  icon?: string;
  assigned?: string;
  ticket?: string;
  priority?: unknown;
  cssClasses?: string;
}

interface KanbanDb {
  clear(): void;
  getSections(): KanbanDbNode[];
}

interface DbModule {
  default: KanbanDb;
  /** The DB's node list, which the appended line of `exposeNodes` returns. */
  __exposeNodes(): KanbanDbNode[];
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string } };
  };
  utils: {
    encodeEntities(text: string): string;
  };
  /** The Jison parser of `kanban.jison`, whose `yy` is the DB. */
  grammar: { yy: unknown; parse(text: string): unknown };
  db: DbModule;
}

/** Appends the accessor of the DB's node list to `kanbanDb.ts`, after its own code is compiled. */
const exposeNodes: Plugin = {
  name: 'expose-kanban-nodes',
  enforce: 'post',
  transform(code, id) {
    if (!id.endsWith('/diagrams/kanban/kanbanDb.ts')) {
      return null;
    }
    return { code: `${code}\nexport const __exposeNodes = () => nodes;\n`, map: null };
  },
};

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [preprocess, utils, grammar, db] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: Modules['grammar'] }>(
      '/packages/mermaid/src/diagrams/kanban/parser/kanban.jison'
    ),
    reference.load<DbModule>('/packages/mermaid/src/diagrams/kanban/kanbanDb.ts'),
  ]);
  return {
    preprocess,
    utils,
    grammar: grammar.parser,
    db: {
      default: db.default,
      __exposeNodes: () => db.__exposeNodes(),
    },
  };
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

await runExporter(
  loadModules,
  (input, { preprocess, utils, grammar, db }) => {
    const { code } = preprocess.preprocessDiagram(input.text);
    const kanbanDb = db.default;
    kanbanDb.clear();
    grammar.yy = kanbanDb;
    grammar.parse(utils.encodeEntities(code.cleaned) + '\n');

    // A section is the same object as its node, so membership in the sections is the flag.
    const sections = new Set<KanbanDbNode>(kanbanDb.getSections());
    return {
      version: 1,
      nodes: db.__exposeNodes().map((node) => ({
        id: node.id,
        level: node.level,
        label: json(node.label),
        parentId: node.parentId,
        isSection: sections.has(node),
        icon: node.icon,
        assigned: node.assigned,
        ticket: node.ticket,
        priority: json(node.priority),
        cssClasses: node.cssClasses,
      })),
    };
  },
  [exposeNodes]
);
