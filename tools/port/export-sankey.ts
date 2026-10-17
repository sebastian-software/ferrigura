/**
 * Canonical JSON of sankey diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram`, then `encodeEntities` and a line break, as
 * `Diagram.fromText` gives it to the Jison parser. The parser's `parse` is wrapped to run
 * `prepareTextForParsing` first (`sankeyDiagram.ts`), so the exporter applies that function itself.
 * The parser fills the DB through its own calls. The DB sanitises every node name and the title
 * with DOMPurify, so the reference runs with a window (`provideDomForSanitising`). A value that is
 * not finite is written as its sentinel string, as `value.rs` writes it.
 *
 * Usage: `tsx tools/port/export-sankey.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface SankeyGraph {
  nodes: { id: string }[];
  links: { source: string; target: string; value: number }[];
}

interface SankeyDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getGraph(): SankeyGraph;
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  utils: {
    encodeEntities(text: string): string;
  };
  sankeyUtils: {
    prepareTextForParsing(text: string): string;
  };
  /** The Jison parser of `sankey.jison`, whose `yy` is the DB. */
  grammar: { yy: unknown; parse(text: string): unknown };
  db: { default: SankeyDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [preprocess, utils, sankeyUtils, grammar, db] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<Modules['sankeyUtils']>('/packages/mermaid/src/diagrams/sankey/sankeyUtils.ts'),
    reference.load<{ parser: Modules['grammar'] }>(
      '/packages/mermaid/src/diagrams/sankey/parser/sankey.jison'
    ),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/sankey/sankeyDB.ts'),
  ]);
  return { preprocess, utils, sankeyUtils, grammar: grammar.parser, db };
}

/** A value as JSON: the number when it is finite, else `"NaN"`, `"Infinity"` or `"-Infinity"`. */
const finite = (value: number): number | string => (Number.isFinite(value) ? value : String(value));

await runExporter(loadModules, (input, { preprocess, utils, sankeyUtils, grammar, db }) => {
  const { code, title } = preprocess.preprocessDiagram(input.text);
  const sankeyDb = db.default;
  sankeyDb.clear();
  grammar.yy = sankeyDb;
  // `Diagram.fromText` sets the front matter title before the parse.
  if (title) {
    sankeyDb.setDiagramTitle(title);
  }
  grammar.parse(sankeyUtils.prepareTextForParsing(utils.encodeEntities(code.cleaned) + '\n'));

  const graph = sankeyDb.getGraph();
  return {
    version: 1,
    title: sankeyDb.getDiagramTitle(),
    accTitle: sankeyDb.getAccTitle(),
    accDescr: sankeyDb.getAccDescription(),
    nodes: graph.nodes.map((node) => ({ id: node.id })),
    links: graph.links.map((link) => ({
      source: link.source,
      target: link.target,
      value: finite(link.value),
    })),
  };
});
