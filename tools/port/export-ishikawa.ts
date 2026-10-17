/**
 * Canonical JSON of ishikawa diagrams, from the TypeScript reference.
 *
 * Each input goes through `preprocessDiagram` and `Diagram.fromText`, as `mermaidAPI.parse` gives
 * it: the reference's own loader makes a new `IshikawaDB` for every diagram, clears the common DB,
 * sets the front matter title, and runs the Jison parser on the cleaned text with its line break.
 * The model is the DB after that parse. The texts keep the entity placeholders the DB keeps. An
 * input the detector does not give to ishikawa is a rejection, as the reference rejects it.
 *
 * Usage: `tsx tools/port/export-ishikawa.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface IshikawaNode {
  text: string;
  children: IshikawaNode[];
}

interface IshikawaDb {
  getRoot(): IshikawaNode | undefined;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: unknown; title?: string };
  };
  diagram: {
    Diagram: {
      fromText(
        code: unknown,
        metadata: { title?: string }
      ): Promise<{ type: string; db: IshikawaDb }>;
    };
  };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [preprocess, diagram] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['diagram']>('/packages/mermaid/src/Diagram.ts'),
  ]);
  return { preprocess, diagram };
}

/** A node as the canonical JSON writes it. */
const tree = (node: IshikawaNode): IshikawaNode => ({
  text: node.text,
  children: node.children.map(tree),
});

await runExporter(loadModules, async (input, { preprocess, diagram }) => {
  const processed = preprocess.preprocessDiagram(input.text);
  const parsed = await diagram.Diagram.fromText(processed.code, { title: processed.title });
  if (parsed.type !== 'ishikawa') {
    throw new Error(`detected as ${parsed.type}, not ishikawa`);
  }
  const db = parsed.db;
  const root = db.getRoot();
  return {
    version: 1,
    title: db.getDiagramTitle(),
    accTitle: db.getAccTitle(),
    accDescr: db.getAccDescription(),
    root: root ? tree(root) : null,
  };
});
