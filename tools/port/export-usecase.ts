/**
 * Canonical JSON of use case diagrams, from the TypeScript reference. The text goes through
 * `preprocessDiagram`, `encodeEntities` and a line break, as `Diagram.fromText` gives it to the
 * Chevrotain parser, which clears the DB and fills it. The DB's header texts go through DOMPurify,
 * so the reference runs with a window (`provideDomForSanitising`). The model is the DB state after
 * the parse: the labels raw, the arrays in the DB's order, the graph AST of `getAST`. The rows of a
 * JSON table are those of `getData` (the only display of the object's numbers and key order). A
 * non-finite number is written as its sentinel string, as `value.rs` writes it.
 *
 * Usage: `tsx tools/port/export-usecase.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface UsecaseRow {
  key: string;
  accessibleKey: string;
  value: string;
}

interface UsecaseDataNode {
  id: string;
  shape: string;
  jsonRows?: UsecaseRow[];
}

interface UsecaseDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getDirection(): string;
  getActors(): ReadonlyMap<string, Record<string, unknown>>;
  getUseCases(): ReadonlyMap<string, Record<string, unknown>>;
  getSystemBoundaries(): ReadonlyMap<string, Record<string, unknown>>;
  getRelationships(): readonly Record<string, unknown>[];
  getNotes(): ReadonlyMap<string, Record<string, unknown>>;
  getJsonNodes(): ReadonlyMap<string, Record<string, unknown>>;
  getClassDefs(): ReadonlyMap<string, Record<string, unknown>>;
  getAST(): unknown;
  getData(): { nodes: UsecaseDataNode[] };
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  utils: {
    encodeEntities(text: string): string;
  };
  parser: { parse(text: string): Promise<void> };
  db: { db: UsecaseDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  const [preprocess, utils, parser, db] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: Modules['parser'] }>(
      '/packages/mermaid/src/diagrams/usecase/parser/usecase.chevrotain.ts'
    ),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/usecase/usecaseDb.ts'),
  ]);
  return { preprocess, utils, parser: parser.parser, db };
}

/** A value as JSON: a non-finite number is its sentinel string, at any depth. */
function sentinels(value: unknown): unknown {
  if (typeof value === 'number') {
    return Number.isFinite(value) ? value : String(value);
  }
  if (Array.isArray(value)) {
    return value.map(sentinels);
  }
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, sentinels(item)]));
  }
  return value;
}

/** An object without its undefined members, as `JSON.stringify` writes it. */
function defined(object: Record<string, unknown>): Record<string, unknown> {
  return Object.fromEntries(Object.entries(object).filter(([, value]) => value !== undefined));
}

await runExporter(loadModules, async (input, { preprocess, utils, parser, db: dbModule }) => {
  const db = dbModule.db;
  const { code, title } = preprocess.preprocessDiagram(input.text);
  // `Diagram.fromText`: the DB is cleared, the front matter title is set, then the parser runs.
  db.clear();
  if (title) {
    db.setDiagramTitle(title);
  }
  await parser.parse(utils.encodeEntities(code.cleaned) + '\n');

  const rows = new Map<string, UsecaseRow[]>();
  for (const node of db.getData().nodes) {
    if (node.shape === 'usecaseJsonTable' && node.jsonRows) {
      rows.set(node.id, node.jsonRows);
    }
  }

  return {
    version: 1,
    title: db.getDiagramTitle(),
    direction: db.getDirection(),
    accTitle: db.getAccTitle(),
    accDescription: db.getAccDescription(),
    actors: [...db.getActors().values()].map((actor) =>
      defined({
        id: actor.id,
        label: actor.label,
        labelType: actor.labelType,
        type: actor.type,
        icon: actor.icon,
        business: actor.business,
        stereotype: actor.stereotype,
        parentId: actor.parentId,
        classes: actor.classes,
        styles: actor.styles,
      })
    ),
    useCases: [...db.getUseCases().values()].map((useCase) =>
      defined({
        id: useCase.id,
        label: useCase.label,
        labelType: useCase.labelType,
        shape: useCase.shape,
        business: useCase.business,
        stereotype: useCase.stereotype,
        parentId: useCase.parentId,
        classes: useCase.classes,
        styles: useCase.styles,
      })
    ),
    systemBoundaries: [...db.getSystemBoundaries().values()].map((boundary) => ({
      id: boundary.id,
      label: boundary.label,
      labelType: boundary.labelType,
      type: boundary.type,
      members: boundary.members,
      classes: boundary.classes,
      styles: boundary.styles,
    })),
    relationships: db.getRelationships().map((relationship) =>
      defined({
        id: relationship.id,
        explicitId: relationship.explicitId,
        source: relationship.source,
        target: relationship.target,
        type: relationship.type,
        arrowType: relationship.arrowType,
        label: relationship.label,
        labelType: relationship.labelType,
        minlen: relationship.minlen,
        classes: relationship.classes,
        styles: relationship.styles,
        animate: relationship.animate,
        animation: relationship.animation,
      })
    ),
    notes: [...db.getNotes().values()].map((note) => ({
      id: note.id,
      target: note.target,
      label: note.label,
      labelType: note.labelType,
    })),
    jsonNodes: [...db.getJsonNodes().values()].map((json) => ({
      id: json.id,
      value: sentinels(json.value),
      propertyOrder: json.propertyOrder,
      classes: json.classes,
      styles: json.styles,
      rows: rows.get(json.id as string) ?? [],
    })),
    classDefs: [...db.getClassDefs().values()].map((definition) => ({
      id: definition.id,
      styles: definition.styles,
    })),
    ast: db.getAST(),
  };
});
