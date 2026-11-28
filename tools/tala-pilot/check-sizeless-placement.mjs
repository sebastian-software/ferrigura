// Compare the live Go sizeless placement stage, saved geometry, and the Rust implementation.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const [reference, tracedReference, rust] = process.argv.slice(2).map((path) => resolve(path));
assert(
  reference && tracedReference && rust,
  'usage: node check-sizeless-placement.mjs <Go> <traced Go> <Rust>'
);

const input = await readFile(
  new URL('captures/observatory-services.measured.json', import.meta.url),
  'utf8'
);
const { edgeLabels, seeds } = await readFixture('sizeless/observatory.json');
const initialization = await readFixture('initialize/fixtures.json');
const { graph } = initialization.find(({ name }) => name === 'Cyclic observatory services');
assert.equal(edgeLabels.length, graph.edges.length);
for (const [index, edge] of graph.edges.entries()) {
  assert.equal(edge.id, edgeLabels[index].id);
  Object.assign(edge, edgeLabels[index]);
}
const options = { input, encoding: 'utf8', maxBuffer: 8_000_000, timeout: 30_000 };
const baseline = execFileSync(reference, ['--default'], options);
const trace = spawnSync(tracedReference, ['--default'], {
  ...options,
  env: { ...process.env, TALA_PILOT_TRACE_SIZELESS: '1' },
});
assert.ifError(trace.error);
assert.equal(trace.status, 0, trace.stderr);
assert.equal(trace.stdout, baseline, 'Tracing changed the final pinned Go layout');

const prefix = 'TALA_PILOT_SIZELESS ';
const captures = trace.stderr
  .split('\n')
  .filter((line) => line.startsWith(prefix))
  .map((line) => JSON.parse(line.slice(prefix.length)));
assert.equal(captures.length, seeds.length * 3, 'Expected all stages for each default seed');

for (const { seed, firstPass, fullPhase, sizedTransition } of seeds) {
  const stages = [
    ['firstPass', '--sizeless-first-pass', firstPass],
    ['fullPhase', '--sizeless-phase', fullPhase],
    ['sizedTransition', '--sized-transition', sizedTransition],
  ];
  for (const [stage, command, expected] of stages) {
    const capture = captures.find((item) => item.seed === seed && item.stage === stage);
    assert(capture, `Missing Go trace for ${stage}, seed ${seed}`);
    assert.deepEqual(capture.graph, graph, `${stage}, seed ${seed}: placement input changed`);
    assert.deepEqual(
      capture.expected,
      expected,
      `${stage}, seed ${seed}: saved Go geometry changed`
    );
    const run = execFileSync(rust, [command], {
      ...options,
      input: JSON.stringify({ graph, seed }),
    });
    assert.deepEqual(JSON.parse(run), expected, `${stage}, seed ${seed}: native geometry differs`);
  }
}
console.log(`Matched ${captures.length} seeded placement stages with pinned Go.`);

async function readFixture(path) {
  const source = new URL('../../crates/ferrigura-tala/src/', import.meta.url);
  return JSON.parse(await readFile(new URL(path, source), 'utf8'));
}
