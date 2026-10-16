/**
 * Differential run for the YAML reader: the front matter of every `.mmd` fixture, the flowchart
 * shape data (`@{ … }`) of the fixtures and of the flowchart specs, and the explicit cases of
 * `yaml-cases.ts` go through `js-yaml` (`export-yaml.ts`) and through ferrigura
 * (`cargo run --example yaml`). Values are compared in their canonical form; a rejection on both
 * sides counts as agreement.
 *
 * Usage: `pnpm port:diff yaml`
 */
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { matchFrontMatter } from '../../packages/mermaid/src/diagram-api/regexes.ts';
import { yamlCases } from './yaml-cases.ts';

interface YamlInput {
  id: number;
  source: string;
  text: string;
}

interface YamlOutput {
  id: number;
  ok: boolean;
  value?: unknown;
  error?: string;
}

const root = fileURLToPath(new URL('../../', import.meta.url));
const workDir = join(root, 'target/port');
const inputPath = join(workDir, 'yaml-inputs.json');
const jsPath = join(workDir, 'yaml-js.json');
const rsPath = join(workDir, 'yaml-rs.json');

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
  return found;
}

/** The YAML body of a fixture's front matter, with the indent removed as `extractFrontMatter` does. */
function frontMatterBody(text: string): string | undefined {
  const normalized = text.replace(/\r\n?/g, '\n');
  const match = matchFrontMatter(normalized);
  if (!match) return undefined;
  const { indent, body } = match;
  return indent
    ? body
        .split('\n')
        .map((line) => (line.startsWith(indent) ? line.slice(indent.length) : line))
        .join('\n')
    : body;
}

/** Equality of two canonical JSON values; numbers compare with `===`, so -0 and 0 are equal. */
function same(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  const keysA = Object.keys(a);
  const keysB = Object.keys(b);
  if (keysA.length !== keysB.length) return false;
  return keysA.every(
    (key) =>
      Object.hasOwn(b, key) &&
      same((a as Record<string, unknown>)[key], (b as Record<string, unknown>)[key])
  );
}

const inputs: YamlInput[] = [];
const fixtures = await filesUnder(join(root, 'e2e'), '.mmd');
const flowchartSpecs = await filesUnder(
  join(root, 'packages/mermaid/src/diagrams/flowchart'),
  '.spec.ts'
);
const shapeData = /@{[^\n{}]*}/g;

for (const path of fixtures) {
  const text = await readFile(path, 'utf8');
  const body = frontMatterBody(text);
  if (body !== undefined) {
    inputs.push({ id: 0, source: `front matter: ${path}`, text: body });
  }
  for (const match of text.match(shapeData) ?? []) {
    inputs.push({ id: 0, source: `shape data: ${path}`, text: match.slice(1) });
  }
}
for (const path of flowchartSpecs) {
  const text = await readFile(path, 'utf8');
  for (const match of text.match(shapeData) ?? []) {
    inputs.push({ id: 0, source: `shape data: ${path}`, text: match.slice(1) });
  }
}
for (const text of yamlCases) {
  inputs.push({ id: 0, source: 'yaml case', text });
}

const numbered = inputs.map((input, id) => ({ ...input, id }));
await mkdir(workDir, { recursive: true });
await writeFile(inputPath, JSON.stringify(numbered, null, 2));

run('pnpm', ['exec', 'tsx', 'tools/port/export-yaml.ts', inputPath, jsPath]);
run('cargo', [
  'run',
  '--quiet',
  '-p',
  'ferrigura-syntax',
  '--example',
  'yaml',
  '--',
  inputPath,
  rsPath,
]);

const js = JSON.parse(await readFile(jsPath, 'utf8')) as YamlOutput[];
const rs = JSON.parse(await readFile(rsPath, 'utf8')) as YamlOutput[];
const jsById = new Map(js.map((output) => [output.id, output]));
const rsById = new Map(rs.map((output) => [output.id, output]));

let accepted = 0;
let differences = 0;
for (const input of numbered) {
  const jsOutput = jsById.get(input.id);
  const rsOutput = rsById.get(input.id);
  if (jsOutput?.ok) accepted += 1;
  const agree =
    jsOutput !== undefined &&
    rsOutput !== undefined &&
    jsOutput.ok === rsOutput.ok &&
    (!jsOutput.ok || same(jsOutput.value, rsOutput.value));
  if (agree) continue;
  differences += 1;
  console.log(`\n✗ ${input.source}`);
  console.log(`  input: ${JSON.stringify(input.text.slice(0, 300))}`);
  console.log(`  js: ${JSON.stringify(jsOutput).slice(0, 600)}`);
  console.log(`  rs: ${JSON.stringify(rsOutput).slice(0, 600)}`);
}

console.log(
  `\nyaml: ${numbered.length} inputs, ${accepted} accepted by js-yaml, ${differences} differences`
);

await writeFile(
  join(workDir, 'yaml-summary.json'),
  JSON.stringify({ type: 'yaml', inputs: numbered.length, accepted, differences }, null, 2)
);

if (differences > 0) {
  process.exit(1);
}
