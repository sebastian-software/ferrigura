// Replay the winning Go candidate's routes through Ferrigura's native adapter.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFile, readdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { parseArgs } from 'node:util';

const { values } = parseArgs({
  options: {
    reference: { type: 'string' },
    trace: { type: 'string' },
    rust: { type: 'string' },
    write: { type: 'boolean' },
  },
});
assert(values.reference && values.trace && values.rust, 'Pass --reference, --trace and --rust.');
const reference = resolve(values.reference);
const tracedReference = resolve(values.trace);
const rust = resolve(values.rust);
const captures = new URL('captures/', import.meta.url);
const fixturePath = new URL(
  '../../crates/ferrigura-tala/src/route_output/fixtures.json',
  import.meta.url
);
const files = (await readdir(captures)).filter((file) => file.endsWith('.measured.json')).sort();
assert(files.length > 0, 'No measured flowcharts');
const prefix = 'TALA_PILOT_ROUTE_OUTPUT ';
const options = { encoding: 'utf8', maxBuffer: 8_000_000, timeout: 60_000 };
let routesCompared = 0;
let reversedRoutes = 0;
const snapshots = [];

for (const file of files) {
  const name = file.replace(/\.measured\.json$/, '');
  const input = await readFile(new URL(file, captures), 'utf8');
  compare(name, input);

  const reversed = JSON.parse(input);
  const edge = reversed.edges.find((item) => item.targetArrow && !item.sourceArrow);
  if (edge) {
    [edge.source, edge.target] = [edge.target, edge.source];
    edge.sourceArrow = true;
    edge.targetArrow = false;
    compare(`${name} with a source-only arrow`, JSON.stringify(reversed));
  }
}
assert(reversedRoutes > 0, 'No source-only arrows were traced');
assert.equal(snapshots.length, 2, 'Expected two route projection fixtures');
if (values.write) {
  await writeFile(fixturePath, JSON.stringify(snapshots, null, 2) + '\n');
} else {
  const stored = JSON.parse(await readFile(fixturePath, 'utf8'));
  assert.deepEqual(snapshots, stored, 'Checked-in route projection fixtures differ from fresh Go');
}
console.log(
  `Matched ${routesCompared} route projections with Go, including ${reversedRoutes} reversed routes.`
);

function compare(name, input) {
  const plain = execFileSync(reference, ['--default'], { ...options, input });
  const trace = spawnSync(tracedReference, ['--default'], {
    ...options,
    input,
    env: { ...process.env, TALA_PILOT_TRACE_ROUTE_OUTPUT: '1' },
  });
  assert.ifError(trace.error);
  assert.equal(trace.status, 0, `${name}: ${trace.stderr}`);
  assert.equal(trace.stdout, plain, `${name}: trace changed the final Go output`);
  const expected = JSON.parse(plain).edges.map(({ id, points, label }) => ({
    id,
    points,
    label: label ?? null,
  }));
  const checkpoints = trace.stderr
    .split('\n')
    .filter((line) => line.startsWith(prefix))
    .map((line, index) => ({ id: expected[index]?.id, ...JSON.parse(line.slice(prefix.length)) }));
  assert.equal(checkpoints.length, expected.length, `${name}: number of projected routes`);
  const actual = execFileSync(rust, ['--route-output'], {
    ...options,
    input: JSON.stringify(checkpoints),
  });
  assert.deepEqual(
    JSON.parse(actual),
    expected,
    `${name}: authored route or label differs from Go`
  );
  if (name === 'observatory-services with a source-only arrow') {
    for (const [index, checkpoint] of checkpoints.entries()) {
      if (checkpoint.reversedForLayout || index === 1) {
        snapshots.push({ input: checkpoint, expected: expected[index] });
      }
    }
  }
  routesCompared += checkpoints.length;
  reversedRoutes += checkpoints.filter(({ reversedForLayout }) => reversedForLayout).length;
}
