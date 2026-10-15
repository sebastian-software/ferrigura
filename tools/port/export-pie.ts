/**
 * Canonical JSON of pie diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram` (front matter, directives, comments), then through
 * the pie parser of `@mermaid-js/parser`, and the slices and `showData` go through the pie DB as
 * `pieParser.ts` does. The DB sanitises the header texts with DOMPurify, which needs a DOM; the
 * canonical model keeps the source text, so the two header rules that the DB applies on top of
 * the grammar are applied here instead.
 *
 * Usage: `tsx tools/port/export-pie.ts inputs.json outputs.json`
 */
import { registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface PieSlice {
  label: string;
  value: number;
}

interface PieAst {
  title?: string;
  accTitle?: string;
  accDescr?: string;
  showData: boolean;
  sections: PieSlice[];
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  parser: { parse(diagramType: 'pie', text: string): Promise<PieAst> };
  db: {
    clear(): void;
    setShowData(toggle: boolean): void;
    addSection(section: PieSlice): void;
    getSections(): Map<string, number>;
    getShowData(): boolean;
  };
}

async function loadModules(reference: Reference): Promise<Modules> {
  await registerDetectors(reference);
  return Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['parser']>('/packages/parser/src/parse.ts'),
    reference.load<{ db: Modules['db'] }>('/packages/mermaid/src/diagrams/pie/pieDb.ts'),
  ]).then(([preprocess, parser, pieDb]) => ({ preprocess, parser, db: pieDb.db }));
}

await runExporter(loadModules, async (input, { preprocess, parser, db }) => {
  const { code, title: frontMatterTitle } = preprocess.preprocessDiagram(input.text);
  const ast = await parser.parse('pie', code.cleaned);

  db.clear();
  db.setShowData(ast.showData);
  for (const slice of ast.sections) {
    db.addSection(slice);
  }

  return {
    version: 1,
    title: ast.title || frontMatterTitle || '',
    accTitle: ast.accTitle ? ast.accTitle.replace(/^\s+/g, '') : '',
    accDescr: ast.accDescr ? ast.accDescr.replace(/\n\s+/g, '\n') : '',
    showData: db.getShowData(),
    sections: [...db.getSections()].map(([label, value]) => ({ label, value })),
  };
});
