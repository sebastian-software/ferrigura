/**
 * Canonical JSON of venn diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram`, then `encodeEntities` and a line break, as
 * `Diagram.fromText` gives it to the Jison parser. The parser fills the venn DB through its own
 * calls, and the DB sanitises the title, so the reference runs with a window
 * (`provideDomForSanitising`). The DB's texts are written as the DB holds them, with the entity
 * placeholders kept, as `docs-port/canonical-model.md` says for the other types. A size that is not
 * finite is written as its sentinel string, as `value.rs` writes it, since `JSON.stringify` would
 * write `NaN` as `null`.
 *
 * Usage: `tsx tools/port/export-venn.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface VennSubsetData {
  sets: string[];
  size: number;
  label?: string;
}

interface VennTextData {
  sets: string[];
  id: string;
  label?: string;
}

interface VennStyleData {
  targets: string[];
  styles: Record<string, string>;
}

interface VennDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getSubsetData(): VennSubsetData[];
  getTextData(): VennTextData[];
  getStyleData(): VennStyleData[];
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  utils: {
    encodeEntities(text: string): string;
  };
  /** The Jison parser of `venn.jison`, whose `yy` is the DB. */
  grammar: { yy: unknown; parse(text: string): unknown };
  db: { db: VennDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [preprocess, utils, grammar, db] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: Modules['grammar'] }>(
      '/packages/mermaid/src/diagrams/venn/parser/venn.jison'
    ),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/venn/vennDB.ts'),
  ]);
  return { preprocess, utils, grammar: grammar.parser, db };
}

/** A size as JSON: the number when it is finite, else `"NaN"`, `"Infinity"` or `"-Infinity"`. */
const size = (value: number): number | string => (Number.isFinite(value) ? value : String(value));

await runExporter(loadModules, (input, { preprocess, utils, grammar, db }) => {
  const { code, title } = preprocess.preprocessDiagram(input.text);
  const vennDb = db.db;
  vennDb.clear();
  grammar.yy = vennDb;
  // `Diagram.fromText` sets the front matter title before the parse.
  if (title) {
    vennDb.setDiagramTitle(title);
  }
  grammar.parse(utils.encodeEntities(code.cleaned) + '\n');

  return {
    version: 1,
    title: vennDb.getDiagramTitle(),
    subsets: vennDb.getSubsetData().map((subset) => ({
      sets: subset.sets,
      size: size(subset.size),
      label: subset.label ?? '',
    })),
    textNodes: vennDb.getTextData().map((node) => ({
      sets: node.sets,
      id: node.id,
      label: node.label ?? '',
    })),
    styles: vennDb.getStyleData().map((entry) => ({
      targets: entry.targets,
      styles: Object.entries(entry.styles),
    })),
  };
});
