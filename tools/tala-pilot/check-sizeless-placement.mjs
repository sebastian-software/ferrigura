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
const measuredGraph = JSON.parse(input);
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
assert.equal(captures.length, seeds.length * 7, 'Expected all stages for each default seed');

for (const {
  seed,
  firstPass,
  fullPhase,
  sizedTransition,
  firstSizedCandidates,
  firstSizedEdgeCosts,
  firstSizedMove,
  firstSizedPass,
} of seeds) {
  const stages = [
    ['firstPass', '--sizeless-first-pass', firstPass],
    ['fullPhase', '--sizeless-phase', fullPhase],
    ['sizedTransition', '--sized-transition', sizedTransition],
    ['firstSizedCandidates', '--first-sized-candidates', firstSizedCandidates],
    ['firstSizedEdgeCosts', '--first-sized-edge-costs', firstSizedEdgeCosts],
    ['firstSizedMove', '--first-sized-move', firstSizedMove],
    ['firstSizedPass', '--first-sized-pass', firstSizedPass],
  ];
  for (const [stage, command, expected] of stages) {
    const capture = captures.find((item) => item.seed === seed && item.stage === stage);
    assert(capture, `Missing Go trace for ${stage}, seed ${seed}`);
    assert.deepEqual(capture.graph, graph, `${stage}, seed ${seed}: placement input changed`);
    const compare = stage === 'firstSizedEdgeCosts' ? assertScoresEqual : assert.deepEqual;
    compare(capture.expected, expected, `${stage}, seed ${seed}: saved Go geometry changed`);
    const run = execFileSync(rust, [command], {
      ...options,
      input: JSON.stringify({ graph, seed }),
    });
    compare(JSON.parse(run), capture.expected, `${stage}, seed ${seed}: native geometry differs`);
  }
  for (const [command, expected] of [
    ['--measured-sized-transition', sizedTransition],
    ['--measured-first-sized-pass', firstSizedPass],
  ]) {
    const run = execFileSync(rust, [command], {
      ...options,
      input: JSON.stringify({ graph: measuredGraph, seed }),
    });
    assert.deepEqual(
      JSON.parse(run),
      expected,
      `${command}: measured input differs for seed ${seed}`
    );
  }
}
console.log(
  `Matched ${captures.length} seeded placement stages and ${seeds.length * 2} measured-input stages with pinned Go.`
);

async function readFixture(path) {
  const source = new URL('../../crates/ferrigura-tala/src/', import.meta.url);
  return JSON.parse(await readFile(new URL(path, source), 'utf8'));
}

function assertScoresEqual(actual, expected, message) {
  const { turnCost: actualTurn, edgeCosts: actualCosts, ...actualExact } = actual;
  const { turnCost: expectedTurn, edgeCosts: expectedCosts, ...expectedExact } = expected;
  assert.deepEqual(actualExact, expectedExact, message);
  assertNumericScore(actualTurn, expectedTurn, message);
  assert.equal(actualCosts.length, expectedCosts.length, message);
  for (const [index, cost] of actualCosts.entries()) {
    assertNumericScore(cost, expectedCosts[index], `${message}, candidate ${index}`);
  }
}

function assertNumericScore(actual, expected, message) {
  if (actual === null || expected === null) {
    assert.equal(actual, expected, message);
    return;
  }
  // Allow only floating-point rounding differences, well below Go's 0.0001 score threshold.
  const tolerance = 4 * Number.EPSILON * Math.max(Math.abs(actual), Math.abs(expected), 1);
  assert(Math.abs(actual - expected) <= tolerance, `${message}: ${actual} vs ${expected}`);
}
