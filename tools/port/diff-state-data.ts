/**
 * Differential run for the layout data of state diagrams: every state input goes through the
 * reference's `getData()` (`export-state-data.ts`) and through ferrigura's `getData` (the
 * `state_data` example of `ferrigura-render`), and the two are compared field by field. A
 * rejection on both sides counts as agreement.
 *
 * Each difference is printed with the path of the field, both values, and the number of differences
 * per field name at the end, so a shared cause shows up as one line.
 *
 * Usage: `pnpm port:diff state-data`
 */
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { collectInputs } from './inputs.ts';
import { exceptions } from './exceptions.ts';
import type { PortInput, PortOutput } from './reference.ts';

const root = fileURLToPath(new URL('../../', import.meta.url));
const workDir = `${root}target/port`;
const inputPath = `${workDir}/state-data-inputs.json`;
const jsPath = `${workDir}/state-data-js.json`;
const rsPath = `${workDir}/state-data-rs.json`;

function run(command: string, args: string[]): void {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit' });
  if (result.status !== 0) {
    console.error(`${command} ${args.join(' ')} failed`);
    process.exit(2);
  }
}

/** Equality of two JSON values; numbers compare with `===`, so -0 and 0 are equal. */
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

interface Leaf {
  path: string;
  js: unknown;
  rs: unknown;
}

/** The fields at which two values differ, with their paths. A list of another length is one leaf. */
function leaves(a: unknown, b: unknown, path: string, out: Leaf[]): void {
  if (same(a, b)) return;
  const bothObjects = typeof a === 'object' && a !== null && typeof b === 'object' && b !== null;
  if (bothObjects && Array.isArray(a) && Array.isArray(b) && a.length === b.length) {
    a.forEach((item, index) => leaves(item, b[index], `${path}[${index}]`, out));
    return;
  }
  if (bothObjects && !Array.isArray(a) && !Array.isArray(b)) {
    const record = a as Record<string, unknown>;
    const other = b as Record<string, unknown>;
    for (const key of new Set([...Object.keys(record), ...Object.keys(other)])) {
      leaves(record[key], other[key], path ? `${path}.${key}` : key, out);
    }
    return;
  }
  out.push({ path, js: a, rs: b });
}

/** The name of a field without its list indexes, as the summary counts them. */
function fieldName(path: string): string {
  return (
    path
      .replace(/\[\d+]/g, '')
      .split('.')
      .pop() ?? path
  );
}

const stateInputs = await collectInputs('state');
await mkdir(workDir, { recursive: true });
await writeFile(inputPath, JSON.stringify(stateInputs, null, 2));

run('pnpm', ['exec', 'tsx', 'tools/port/export-state-data.ts', inputPath, jsPath]);
run('cargo', [
  'run',
  '--quiet',
  '-p',
  'ferrigura-render',
  '--example',
  'state_data',
  '--',
  inputPath,
  rsPath,
]);

const js = JSON.parse(await readFile(jsPath, 'utf8')) as PortOutput[];
const rs = JSON.parse(await readFile(rsPath, 'utf8')) as PortOutput[];
const jsById = new Map(js.map((output) => [output.id, output]));
const rsById = new Map(rs.map((output) => [output.id, output]));

const named = new Map((exceptions['state-data'] ?? []).map((entry) => [entry.text, entry.reason]));
let accepted = 0;
let excepted = 0;
const stale: string[] = [];
let differing = 0;
const fieldCounts = new Map<string, number>();
for (const input of stateInputs as PortInput[]) {
  const jsOutput = jsById.get(input.id);
  const rsOutput = rsById.get(input.id);
  if (jsOutput?.ok) accepted += 1;
  if (jsOutput === undefined || rsOutput === undefined) {
    differing += 1;
    console.log(`\n✗ ${input.source}: missing output`);
    continue;
  }

  let agree = jsOutput.ok === rsOutput.ok;
  const found: Leaf[] = [];
  if (agree && jsOutput.ok && rsOutput.ok && !same(jsOutput.model, rsOutput.model)) {
    agree = false;
    leaves(jsOutput.model, rsOutput.model, '', found);
  }

  if (named.has(input.text)) {
    if (agree) stale.push(`${input.source}: ${named.get(input.text)}`);
    else excepted += 1;
    continue;
  }
  if (agree) continue;

  differing += 1;
  console.log(`\n✗ ${input.source}`);
  console.log(`  input: ${JSON.stringify(input.text.slice(0, 300))}`);
  if (jsOutput.ok !== rsOutput.ok) {
    console.log(`  js: ${JSON.stringify(jsOutput).slice(0, 300)}`);
    console.log(`  rs: ${JSON.stringify(rsOutput).slice(0, 300)}`);
    continue;
  }
  console.log(`  ${found.length} differing fields`);
  for (const leaf of found) {
    const name = fieldName(leaf.path);
    fieldCounts.set(name, (fieldCounts.get(name) ?? 0) + 1);
    if (fieldCounts.get(name) === 1 || found.length <= 5) {
      console.log(`  ${leaf.path}: js=${JSON.stringify(leaf.js)} rs=${JSON.stringify(leaf.rs)}`);
    }
  }
}

for (const entry of stale) {
  console.log(`\n✗ stale exception, now agrees: ${entry}`);
  differing += 1;
}
if (fieldCounts.size > 0) {
  console.log('\ndifferences by field (the first five of each input are printed above):');
  for (const [field, count] of [...fieldCounts].sort((a, b) => b[1] - a[1])) {
    console.log(`  ${field}: ${count}`);
  }
}

console.log(
  `\nstate-data: ${stateInputs.length} inputs, ${accepted} accepted by the reference, ` +
    `${excepted} named exceptions, ${differing} differences (${stale.length} stale exceptions)`
);

await writeFile(
  `${workDir}/state-data-summary.json`,
  JSON.stringify(
    {
      type: 'state-data',
      inputs: stateInputs.length,
      accepted,
      excepted,
      stale: stale.length,
      differences: differing,
    },
    null,
    2
  )
);

if (differing > 0) {
  process.exitCode = 1;
}
