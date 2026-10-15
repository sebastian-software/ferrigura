/**
 * Checks the header rules of `pie-header.ts` against the reference's pie DB setters.
 *
 * For every pie input the reference accepts, the header is read from the real DB as
 * `Diagram.fromText` fills it: the front matter title first, then `pieParser.parse`, which calls
 * the setters. The setters sanitise with DOMPurify, which needs a window, so jsdom provides one
 * before the reference is loaded. Inputs whose header texts contain `&`, `<`, `>` or a no-break
 * space are skipped, since DOMPurify serialises those as HTML; every other input must agree.
 *
 * Usage: `pnpm port:check-header`
 */
import { JSDOM } from 'jsdom';

import { collectInputs } from './inputs.ts';
import { pieHeader, type PieHeaderAst } from './pie-header.ts';
import { openReference, registerDetectors } from './reference.ts';

const dom = new JSDOM('<!doctype html><html><body></body></html>');
Object.assign(globalThis, { window: dom.window, document: dom.window.document });

interface PreprocessModule {
  preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
}

interface ParserModule {
  parse(diagramType: 'pie', text: string): Promise<PieHeaderAst>;
}

interface PieDbModule {
  db: {
    clear(): void;
    setDiagramTitle(title: string): void;
    getDiagramTitle(): string;
    getAccTitle(): string;
    getAccDescription(): string;
  };
}

interface PieParserModule {
  parser: { parse(text: string): Promise<void> };
}

const htmlSpecial = /[&<>\u00a0]/;

const reference = await openReference();
try {
  await registerDetectors(reference);
  const preprocess = await reference.load<PreprocessModule>('/packages/mermaid/src/preprocess.ts');
  const parser = await reference.load<ParserModule>('/packages/parser/src/parse.ts');
  const pieDb = await reference.load<PieDbModule>('/packages/mermaid/src/diagrams/pie/pieDb.ts');
  const pieParser = await reference.load<PieParserModule>(
    '/packages/mermaid/src/diagrams/pie/pieParser.ts'
  );

  const inputs = await collectInputs('pie');
  let checked = 0;
  let skipped = 0;
  let rejected = 0;
  const mismatches: string[] = [];

  for (const input of inputs) {
    const { code, title: frontMatterTitle } = preprocess.preprocessDiagram(input.text);

    let ast: PieHeaderAst;
    try {
      ast = await parser.parse('pie', code.cleaned);
    } catch {
      rejected += 1;
      continue;
    }

    const texts = [ast.title, ast.accTitle, ast.accDescr, frontMatterTitle].filter(Boolean);
    if (texts.some((text) => htmlSpecial.test(text))) {
      skipped += 1;
      continue;
    }

    // `Diagram.fromText`: the front matter title, then the parser, which overwrites the title
    // only when the grammar gives a non-empty one.
    pieDb.db.clear();
    if (frontMatterTitle) {
      pieDb.db.setDiagramTitle(frontMatterTitle);
    }
    try {
      await pieParser.parser.parse(code.cleaned);
    } catch {
      // The DB rejects a negative slice value after the grammar has accepted the text.
      rejected += 1;
      continue;
    }
    const actual = {
      title: pieDb.db.getDiagramTitle(),
      accTitle: pieDb.db.getAccTitle(),
      accDescr: pieDb.db.getAccDescription(),
    };

    const expected = pieHeader(ast, frontMatterTitle);
    checked += 1;
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
      mismatches.push(
        `${input.source}: ${JSON.stringify(input.text.slice(0, 120))}\n` +
          `  DB:     ${JSON.stringify(actual)}\n  rules:  ${JSON.stringify(expected)}`
      );
    }
  }

  for (const mismatch of mismatches) {
    console.log(`✗ ${mismatch}`);
  }
  console.log(
    `pie header: ${checked} checked against the DB, ${mismatches.length} differ; ` +
      `${skipped} skipped for HTML-special text, ${rejected} rejected by the reference`
  );
  if (mismatches.length > 0) {
    process.exitCode = 1;
  }
} finally {
  await reference.close();
  dom.window.close();
}
