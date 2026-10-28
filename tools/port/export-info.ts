/**
 * Canonical JSON of info diagrams, from the TypeScript reference.
 *
 * The reference's info DB holds only the Mermaid version, which the input does not contain, and
 * its parser copies nothing into the DB. The model therefore takes the header that the grammar
 * gives, after `preprocessDiagram`, and carries no front matter title, as the reference's info
 * DB has no title setter.
 *
 * Usage: `tsx tools/port/export-info.ts inputs.json outputs.json`
 */
import { registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface InfoAst {
  title?: string;
  accTitle?: string;
  accDescr?: string;
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string } };
  };
  parser: { parse(diagramType: 'info', text: string): Promise<InfoAst> };
}

async function loadModules(reference: Reference): Promise<Modules> {
  await registerDetectors(reference);
  return Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['parser']>('/packages/parser/src/parse.ts'),
  ]).then(([preprocess, parser]) => ({ preprocess, parser }));
}

await runExporter(loadModules, async (input, { preprocess, parser }) => {
  const { code } = preprocess.preprocessDiagram(input.text);
  const ast = await parser.parse('info', code.cleaned);

  return {
    version: 1,
    title: ast.title ?? '',
    accTitle: ast.accTitle ?? '',
    accDescr: ast.accDescr ?? '',
  };
});
