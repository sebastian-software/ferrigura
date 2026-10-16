/**
 * Differential run for one diagram type: every collected input goes through the TypeScript
 * reference (`export-<type>.ts`) and through ferrigura (`cargo run --example canonical`), and the
 * canonical models are compared. A rejection on both sides counts as agreement; any other
 * difference is reported with both results.
 *
 * Usage: `pnpm port:diff pie` or `pnpm port:diff info`
 */
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { collectInputs } from './inputs.ts';
import type { PortInput, PortOutput } from './reference.ts';

const type = process.argv[2];
if (type === 'config' || type === 'yaml' || type === 'theme') {
  const subRun = spawnSync(
    'pnpm',
    ['exec', 'tsx', fileURLToPath(new URL(`./diff-${type}.ts`, import.meta.url))],
    { stdio: 'inherit' }
  );
  process.exit(subRun.status ?? 2);
}
if (type !== 'pie' && type !== 'info') {
  console.error('usage: pnpm port:diff pie | info | config | yaml | theme');
  process.exit(2);
}

const root = fileURLToPath(new URL('../../', import.meta.url));
const workDir = `${root}target/port`;
const inputPath = `${workDir}/${type}-inputs.json`;
const jsPath = `${workDir}/${type}-js.json`;
const rsPath = `${workDir}/${type}-rs.json`;

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

const inputs = await collectInputs(type);
await mkdir(workDir, { recursive: true });
await writeFile(inputPath, JSON.stringify(inputs, null, 2));

run('pnpm', ['exec', 'tsx', `tools/port/export-${type}.ts`, inputPath, jsPath]);
run('cargo', [
  'run',
  '--quiet',
  '-p',
  'ferrigura-syntax',
  '--example',
  'canonical',
  '--',
  type,
  inputPath,
  rsPath,
]);

const js = JSON.parse(await readFile(jsPath, 'utf8')) as PortOutput[];
const rs = JSON.parse(await readFile(rsPath, 'utf8')) as PortOutput[];
const jsById = new Map(js.map((output) => [output.id, output]));
const rsById = new Map(rs.map((output) => [output.id, output]));

interface Difference {
  source: string;
  text: string;
  js: PortOutput | undefined;
  rs: PortOutput | undefined;
}

const differences: Difference[] = [];
let accepted = 0;
for (const input of inputs as PortInput[]) {
  const jsOutput = jsById.get(input.id);
  const rsOutput = rsById.get(input.id);
  if (jsOutput?.ok) {
    accepted += 1;
  }
  const agree =
    jsOutput !== undefined &&
    rsOutput !== undefined &&
    jsOutput.ok === rsOutput.ok &&
    (!jsOutput.ok || same(jsOutput.model, rsOutput.model));
  if (!agree) {
    differences.push({ source: input.source, text: input.text, js: jsOutput, rs: rsOutput });
  }
}

for (const difference of differences) {
  console.log(`\n✗ ${difference.source}`);
  console.log(`  input: ${JSON.stringify(difference.text.slice(0, 300))}`);
  console.log(`  js: ${JSON.stringify(difference.js)}`);
  console.log(`  rs: ${JSON.stringify(difference.rs)}`);
}

console.log(
  `\n${type}: ${inputs.length} inputs, ${accepted} accepted by the reference, ` +
    `${differences.length} differences`
);

await writeFile(
  `${workDir}/${type}-summary.json`,
  JSON.stringify(
    { type, inputs: inputs.length, accepted, differences: differences.length },
    null,
    2
  )
);

if (differences.length > 0) {
  process.exitCode = 1;
}
