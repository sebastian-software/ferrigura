/**
 * Differential run for Markdown labels: the reference's `markdownToLines` on marked
 * (`export-markdown.ts`) against ferrigura's (`cargo run --example markdown_lines`). The inputs are
 * the Markdown strings (`"`…`"`) of every `.mmd` fixture, the explicit cases of
 * `markdown-cases.ts`, and a seeded set of generated labels that mix the characters marked reads.
 * The lines and words must be the same, with their types.
 *
 * Usage: `pnpm -C tools/port port:diff markdown`
 */
import { spawnSync } from 'node:child_process';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { markdownCases } from './markdown-cases.ts';

interface MarkdownInput {
  id: number;
  source: string;
  text: string;
}

interface MarkdownOutput {
  id: number;
  lines: { content: string; type: string }[][];
}

const root = fileURLToPath(new URL('../../', import.meta.url));
const workDir = join(root, 'target/port');
const inputPath = join(workDir, 'markdown-inputs.json');
const jsPath = join(workDir, 'markdown-js.json');
const rsPath = join(workDir, 'markdown-rs.json');

function run(command: string, args: string[]): void {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit' });
  if (result.status !== 0) {
    console.error(`${command} ${args.join(' ')} failed`);
    process.exit(2);
  }
}

async function filesUnder(dir: string, extension: string): Promise<string[]> {
  const found: string[] = [];
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      found.push(...(await filesUnder(path, extension)));
    } else if (entry.name.endsWith(extension)) {
      found.push(path);
    }
  }
  return found.sort();
}

/** The pieces the generated labels are made of: text, delimiters, breaks and block markers. */
const pieces = [
  'a',
  'b',
  'word',
  'x y',
  ' ',
  '  ',
  '\n',
  '\t',
  '*',
  '**',
  '***',
  '_',
  '__',
  '`',
  '~',
  '~~',
  '\\',
  '\\*',
  '<b>',
  '</b>',
  '<br/>',
  '<br>',
  '<',
  '>',
  '[l](u)',
  '[x]',
  '!',
  '(',
  ')',
  '"',
  "'",
  '#',
  '# ',
  '- ',
  '+ ',
  '1. ',
  '> ',
  '---',
  '===',
  '|',
  '```',
  '    ',
  'www.x.com',
  'https://a.b',
  '&amp;',
  '&#39;',
  'é',
  '中文',
  '.',
  ',',
];

/** mulberry32: a small seeded generator, so the generated set is the same on every run. */
function generator(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function generated(count: number): string[] {
  const random = generator(0x6d61726b);
  const labels: string[] = [];
  for (let index = 0; index < count; index++) {
    const length = 1 + Math.floor(random() * 12);
    let label = '';
    for (let piece = 0; piece < length; piece++) {
      label += pieces[Math.floor(random() * pieces.length)];
    }
    labels.push(label);
  }
  return labels;
}

async function collectInputs(): Promise<MarkdownInput[]> {
  const inputs: MarkdownInput[] = [];
  const seen = new Set<string>();
  const add = (source: string, text: string) => {
    if (seen.has(text)) return;
    seen.add(text);
    inputs.push({ id: inputs.length, source, text });
  };
  for (const file of await filesUnder(join(root, 'e2e'), '.mmd')) {
    const text = await readFile(file, 'utf8');
    for (const match of text.matchAll(/"`([\S\s]*?)`"/g)) {
      add(file.slice(root.length), match[1]);
    }
  }
  markdownCases.forEach((text, index) => add(`markdown-cases.ts #${index}`, text));
  generated(5000).forEach((text, index) => add(`generated #${index}`, text));
  return inputs;
}

const inputs = await collectInputs();
await mkdir(workDir, { recursive: true });
await writeFile(inputPath, JSON.stringify(inputs, null, 2));

run('pnpm', ['-C', 'tools/port', 'exec', 'tsx', 'export-markdown.ts', inputPath, jsPath]);
run('cargo', [
  'run',
  '--quiet',
  '-p',
  'ferrigura-render',
  '--example',
  'markdown_lines',
  '--',
  inputPath,
  rsPath,
]);

const js = new Map(
  (JSON.parse(await readFile(jsPath, 'utf8')) as MarkdownOutput[]).map((o) => [o.id, o])
);
const rs = new Map(
  (JSON.parse(await readFile(rsPath, 'utf8')) as MarkdownOutput[]).map((o) => [o.id, o])
);

let differences = 0;
for (const input of inputs) {
  const jsLines = JSON.stringify(js.get(input.id)?.lines);
  const rsLines = JSON.stringify(rs.get(input.id)?.lines);
  if (jsLines === rsLines) continue;
  differences++;
  if (differences <= 40) {
    console.log(`\n✗ ${input.source}`);
    console.log(`  input: ${JSON.stringify(input.text)}`);
    console.log(`  js: ${jsLines}`);
    console.log(`  rs: ${rsLines}`);
  }
}
const sources = (prefix: string) =>
  inputs.filter((input) => input.source.startsWith(prefix)).length;
console.log(
  `\nmarkdown: ${inputs.length} inputs (${sources('e2e')} from fixtures, ${sources('markdown-cases')} cases, ${sources('generated')} generated), ${differences} differences`
);
if (differences > 0) process.exit(1);
