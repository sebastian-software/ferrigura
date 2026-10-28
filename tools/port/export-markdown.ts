/**
 * The lines and words of Markdown labels from the reference's `markdownToLines`
 * (`packages/mermaid/src/rendering-util/handle-markdown-text.ts`, on marked), loaded from the
 * reference's checkout (`reference-root.ts`).
 *
 * Usage: `tsx tools/port/export-markdown.ts inputs.json outputs.json`
 */
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

import { referenceRoot, requireReference } from './reference-root.ts';

requireReference();

const module = join(referenceRoot, 'packages/mermaid/src/rendering-util/handle-markdown-text.ts');
const { markdownToLines } = (await import(pathToFileURL(module).href)) as {
  markdownToLines: (markdown: string) => { content: string; type: string }[][];
};

const [inputPath, outputPath] = process.argv.slice(2);
const inputs = JSON.parse(await readFile(inputPath, 'utf8')) as { id: number; text: string }[];
const outputs = inputs.map(({ id, text }) => ({ id, lines: markdownToLines(text) }));
await writeFile(outputPath, JSON.stringify(outputs));
