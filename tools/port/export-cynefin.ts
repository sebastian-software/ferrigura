/**
 * Canonical JSON of cynefin diagrams, from the TypeScript reference.
 *
 * The steps are those of `Diagram.fromText`: the text goes through `preprocessDiagram`, the type is
 * detected on the cleaned text (`detectType`, which rejects a text of no type), the DB is cleared,
 * the front matter title is set, and the cleaned text with `encodeEntities` and a line break goes to
 * the parser of `cynefinParser.ts`, which fills the cynefin DB. The model is the DB's state. The
 * DB's header setters sanitise with DOMPurify, which needs a window.
 *
 * Usage: `tsx tools/port/export-cynefin.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface CynefinDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getDomains(): Map<string, { name: string; items: { label: string }[] }>;
  getTransitions(): { from: string; to: string; label?: string }[];
}

interface Modules {
  detectType: { detectType(text: string): string };
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  utils: { encodeEntities(text: string): string };
  parser: { parser: { parse(text: string): Promise<void> } };
  db: { db: CynefinDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  // DOMPurify takes its window when its module loads, so the window comes first.
  provideDomForSanitising();
  await registerDetectors(reference);
  const [detectType, preprocess, utils, parser, db] = await Promise.all([
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<Modules['parser']>('/packages/mermaid/src/diagrams/cynefin/cynefinParser.ts'),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/cynefin/cynefinDb.ts'),
  ]);
  return { detectType, preprocess, utils, parser, db };
}

await runExporter(loadModules, async (input, { detectType, preprocess, utils, parser, db }) => {
  const processed = preprocess.preprocessDiagram(input.text);
  let diagramType: string | undefined;
  try {
    diagramType = detectType.detectType(processed.code.cleaned);
  } catch {
    diagramType = undefined;
  }
  if (diagramType !== 'cynefin') {
    throw new Error(`not a cynefin diagram (detected: ${diagramType ?? 'none'})`);
  }

  const cynefinDb = db.db;
  cynefinDb.clear();
  // `Diagram.fromText` sets the front matter title before the parse.
  if (processed.title) {
    cynefinDb.setDiagramTitle(processed.title);
  }
  await parser.parser.parse(utils.encodeEntities(processed.code.cleaned) + '\n');

  return {
    version: 1,
    title: cynefinDb.getDiagramTitle(),
    accTitle: cynefinDb.getAccTitle(),
    accDescr: cynefinDb.getAccDescription(),
    domains: [...cynefinDb.getDomains().values()].map((domain) => ({
      name: domain.name,
      items: domain.items.map((item) => item.label),
    })),
    transitions: cynefinDb.getTransitions().map((transition) => ({
      from: transition.from,
      to: transition.to,
      label: transition.label ?? null,
    })),
  };
});
