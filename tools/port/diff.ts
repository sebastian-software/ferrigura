/**
 * Differential run for one diagram type: every collected input goes through the TypeScript
 * reference (`export-<type>.ts`) and through ferrigura (`cargo run --example canonical`), and the
 * canonical models are compared. A rejection on both sides counts as agreement; any other
 * difference is reported with both results.
 *
 * Usage: `pnpm port:diff pie`, `pnpm port:diff info` or `pnpm port:diff flowchart`
 */
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { collectInputs } from './inputs.ts';
import { exceptions } from './exceptions.ts';
import type { PortInput, PortOutput } from './reference.ts';

const type = process.argv[2];
if (type !== 'pie' && type !== 'info' && type !== 'flowchart') {
  console.error('usage: pnpm port:diff pie | info | flowchart');
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
const rs = JSON.parse(await readFile(rsPath, 'utf8')) as (PortOutput & { notYet?: string })[];
const jsById = new Map(js.map((output) => [output.id, output]));
const rsById = new Map(rs.map((output) => [output.id, output]));

interface Difference {
  source: string;
  text: string;
  js: PortOutput | undefined;
  rs: PortOutput | undefined;
}

const named = new Map((exceptions[type] ?? []).map((entry) => [entry.text, entry.reason]));
const differences: Difference[] = [];
const stale: Difference[] = [];
const excepted: Difference[] = [];
const notYet: { source: string; construct: string }[] = [];
let accepted = 0;
for (const input of inputs as PortInput[]) {
  const jsOutput = jsById.get(input.id);
  const rsOutput = rsById.get(input.id);
  if (jsOutput?.ok) {
    accepted += 1;
  }
  // A construct the port has not reached yet is neither agreement nor a difference.
  if (rsOutput?.notYet !== undefined) {
    notYet.push({ source: input.source, construct: rsOutput.notYet });
    continue;
  }
  const agree =
    jsOutput !== undefined &&
    rsOutput !== undefined &&
    jsOutput.ok === rsOutput.ok &&
    (!jsOutput.ok || same(jsOutput.model, rsOutput.model));
  const difference = { source: input.source, text: input.text, js: jsOutput, rs: rsOutput };
  if (named.has(input.text)) {
    (agree ? stale : excepted).push(difference);
    continue;
  }
  if (!agree) {
    differences.push(difference);
  }
}

for (const difference of [...differences, ...stale]) {
  console.log(`\n✗ ${difference.source}`);
  console.log(`  input: ${JSON.stringify(difference.text.slice(0, 300))}`);
  console.log(`  js: ${JSON.stringify(difference.js)}`);
  console.log(`  rs: ${JSON.stringify(difference.rs)}`);
}
for (const difference of stale) {
  console.log(`  stale exception, now agrees: ${named.get(difference.text)}`);
}

const notYetCounts = new Map<string, number>();
for (const entry of notYet) {
  notYetCounts.set(entry.construct, (notYetCounts.get(entry.construct) ?? 0) + 1);
}
if (notYet.length > 0) {
  console.log(`\nnot yet ported: ${notYet.length} inputs`);
  for (const [construct, count] of [...notYetCounts].sort((a, b) => b[1] - a[1])) {
    console.log(`  ${count} × ${construct}`);
  }
}

console.log(
  `\n${type}: ${inputs.length} inputs, ${accepted} accepted by the reference, ` +
    `${notYet.length} not yet ported, ${excepted.length} named exceptions, ` +
    `${differences.length} differences, ${stale.length} stale exceptions`
);

await writeFile(
  `${workDir}/${type}-summary.json`,
  JSON.stringify(
    {
      type,
      inputs: inputs.length,
      accepted,
      notYet: notYet.length,
      excepted: excepted.length,
      differences: differences.length,
      stale: stale.length,
    },
    null,
    2
  )
);

if (differences.length > 0 || stale.length > 0) {
  process.exitCode = 1;
}
