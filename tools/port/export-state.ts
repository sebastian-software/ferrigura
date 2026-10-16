/**
 * Canonical JSON of state diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram`, then `encodeEntities` and a line break, as
 * `Diagram.fromText` gives it to the parser. The Jison parser fills the state DB, and the DB's
 * state is read out as the canonical model (`docs-port/state/PLAN.md`, section 6).
 *
 * Normalisation, documented here: the reference names a trailing concurrency region with
 * `generateId()` (`Math.random`). Every id that such a region gives, and every id built from it
 * (`<id>_start` of a `[*]` inside it), is written as `<generated>`, the placeholder of the port.
 *
 * Usage: `tsx tools/port/export-state.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface StateDb {
  rootDoc: unknown[];
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getDirection(): string;
  getData(): { direction: string };
  getStates(): Map<string, Record<string, unknown>>;
  getRelations(): { id1: string; id2: string; relationTitle: string }[];
  getClasses(): Map<string, { id: string; styles: string[]; textStyles: string[] }>;
  getLinks(): Map<{ id: string }, { url: string; tooltip: string }>;
}

interface Modules {
  detectType: (text: string) => string;
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  encode: { encodeEntities(text: string): string };
  state: {
    diagram: {
      parser: { parser: { yy: unknown }; parse(text: string): unknown };
      db: StateDb;
    };
  };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [detect, preprocess, encode, state] = await Promise.all([
    reference.load<{ detectType(text: string): string }>(
      '/packages/mermaid/src/diagram-api/detectType.ts'
    ),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<Modules['state']>('/packages/mermaid/src/diagrams/state/stateDiagram-v2.ts'),
  ]);
  return {
    detectType: (text: string) => detect.detectType(text),
    preprocess,
    encode,
    state,
  };
}

/** The ids of the trailing regions: dividers whose id is not one the parser numbers. */
function generatedIds(node: unknown, found: Set<string>): void {
  if (Array.isArray(node)) {
    for (const item of node) generatedIds(item, found);
    return;
  }
  if (typeof node !== 'object' || node === null) return;
  const record = node as Record<string, unknown>;
  if (record.stmt === 'state' && record.type === 'divider' && typeof record.id === 'string') {
    if (!/^divider-id-\d+$/.test(record.id)) found.add(record.id);
  }
  for (const value of Object.values(record)) generatedIds(value, found);
}

/** Replaces a generated id, and an id built from it, by the placeholder. */
function normalise(node: unknown, generated: Set<string>): unknown {
  if (typeof node === 'string') {
    for (const id of generated) {
      if (node === id) return '<generated>';
      if (node.startsWith(`${id}_`)) return `<generated>${node.slice(id.length)}`;
    }
    return node;
  }
  if (Array.isArray(node)) return node.map((item) => normalise(item, generated));
  if (typeof node === 'object' && node !== null) {
    return Object.fromEntries(
      Object.entries(node).map(([key, value]) => [key, normalise(value, generated)])
    );
  }
  return node;
}

await runExporter(loadModules, async (input, { detectType, preprocess, encode, state }) => {
  const { code, title } = preprocess.preprocessDiagram(input.text);
  // `Diagram.fromText` detects the type of the cleaned text first; the header is case-sensitive there.
  const type = detectType(code.cleaned);
  if (type !== 'stateDiagram') throw new Error(`not a state diagram: ${type}`);
  const db = state.diagram.db;
  db.clear();
  if (title) db.setDiagramTitle(title);
  state.diagram.parser.parser.yy = db;
  await state.diagram.parser.parse(encode.encodeEntities(code.cleaned) + '\n');

  // The snapshot comes before `getData()`: that call runs `docTranslator` a second time on the same
  // objects, which the draw does too, and the model is the DB after the parse.
  const document = db.rootDoc;
  const generated = new Set<string>();
  generatedIds(document, generated);
  const snapshot = JSON.parse(
    JSON.stringify({
      title: db.getDiagramTitle(),
      accTitle: db.getAccTitle(),
      accDescr: db.getAccDescription(),
      direction: db.getDirection(),
      document,
      states: [...db.getStates().values()].map((entry) => ({
        id: entry.id,
        type: entry.type,
        descriptions: entry.descriptions,
        doc: entry.doc,
        note: entry.note,
        classes: entry.classes,
        styles: entry.styles,
        textStyles: entry.textStyles,
      })),
      relations: db.getRelations().map((relation) => ({
        id1: relation.id1,
        id2: relation.id2,
        relationTitle: relation.relationTitle,
      })),
      classes: [...db.getClasses().values()].map((cls) => ({
        id: cls.id,
        styles: cls.styles,
        textStyles: cls.textStyles,
      })),
      links: [...db.getLinks()].map(([key, value]) => ({
        id: key.id,
        url: value.url,
        tooltip: value.tooltip,
      })),
    })
  );
  const model = { version: 1, ...snapshot, layoutDirection: db.getData().direction };
  return normalise(model, generated);
});
