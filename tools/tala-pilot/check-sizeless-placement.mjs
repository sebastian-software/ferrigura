// Compare the live Go sizeless placement stage, saved geometry, and the Rust implementation.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const [reference, tracedReference, rust, bodyReference] = process.argv
  .slice(2)
  .map((path) => resolve(path));
assert(
  reference && tracedReference && rust,
  'usage: node check-sizeless-placement.mjs <Go> <traced Go> <Rust> [body-traced Go]'
);

const input = await readFile(
  new URL('captures/observatory-services.measured.json', import.meta.url),
  'utf8'
);
const measuredGraph = JSON.parse(input);
const { edgeLabels, obstacleOrder, seeds } = await readFixture('sizeless/observatory.json');
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
  env: {
    ...process.env,
    TALA_PILOT_TRACE_SIZELESS: '1',
    TALA_PILOT_TRACE_SIZED_ITERATIONS: '1',
  },
});
assert.ifError(trace.error);
assert.equal(trace.status, 0, trace.stderr);
assert.equal(trace.stdout, baseline, 'Tracing changed the final pinned Go layout');

const captures = traceEntries(trace.stderr, 'TALA_PILOT_SIZELESS ');
const iterations = traceEntries(trace.stderr, 'TALA_PILOT_SIZED_ITERATION ');
const bodyStages = bodyReference ? collectBodyStages() : [];
assert.equal(captures.length, seeds.length * 11, 'Expected all stages for each default seed');

for (const {
  seed,
  firstPass,
  fullPhase,
  sizedTransition,
  firstSizedCandidates,
  firstSizedEdgeCosts,
  firstSizedMove,
  firstSizedPass,
  firstSizedCompaction,
  sizedPhase,
  directedComponent,
  placedComponent,
  routingBodies,
} of seeds) {
  const stages = [
    ['firstPass', '--sizeless-first-pass', firstPass],
    ['fullPhase', '--sizeless-phase', fullPhase],
    ['sizedTransition', '--sized-transition', sizedTransition],
    ['firstSizedCandidates', '--first-sized-candidates', firstSizedCandidates],
    ['firstSizedEdgeCosts', '--first-sized-edge-costs', firstSizedEdgeCosts],
    ['firstSizedMove', '--first-sized-move', firstSizedMove],
    ['firstSizedPass', '--first-sized-pass', firstSizedPass],
    ['firstSizedCompaction', '--first-sized-compaction', firstSizedCompaction],
    ['sizedPhase', '--sized-phase', sizedPhase],
    ['directedComponent', '--direct-flat-component', directedComponent],
    ['placedComponent', '--place-flat-component', placedComponent],
  ];
  const request = JSON.stringify({ graph, seed, obstacleOrder });
  for (const [stage, command, expected] of stages) {
    const capture = captures.find((item) => item.seed === seed && item.stage === stage);
    assert(capture, `Missing Go trace for ${stage}, seed ${seed}`);
    assert.deepEqual(capture.graph, graph, `${stage}, seed ${seed}: placement input changed`);
    assert.deepEqual(
      capture.obstacleOrder,
      obstacleOrder,
      `${stage}, seed ${seed}: Go obstruction order changed`
    );
    const compare = stage === 'firstSizedEdgeCosts' ? assertScoresEqual : assert.deepEqual;
    compare(capture.expected, expected, `${stage}, seed ${seed}: saved Go geometry changed`);
    const run = execFileSync(rust, [command], {
      ...options,
      input: request,
    });
    compare(JSON.parse(run), capture.expected, `${stage}, seed ${seed}: native geometry differs`);
  }

  for (const [command, expected] of [
    ['--measured-sized-transition', sizedTransition],
    ['--measured-first-sized-pass', firstSizedPass],
    ['--measured-sized-phase', sizedPhase],
    ['--measured-flat-placement', placedComponent],
    ['--measured-flat-refinement', routingBodies],
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

  const expected = iterations.filter((entry) => entry.seed === seed).map((entry) => entry.value);
  assert(expected.length > 0, `Missing Go sized-optimizer snapshots for seed ${seed}`);
  const actual = execFileSync(rust, ['--sized-phase-snapshots'], { ...options, input: request });
  assert.deepEqual(
    JSON.parse(actual),
    expected,
    `Sized-optimizer coordinates differ for seed ${seed}`
  );
  if (bodyReference) {
    checkBodyStages(seed, request, placedComponent, routingBodies);
  }
}
console.log(
  `Matched ${captures.length} seeded placement stages and ${iterations.length} sized-optimizer snapshots with pinned Go. The measured input matches ${seeds.length * 5} times.`
);

function traceEntries(output, prefix) {
  return output
    .split('\n')
    .filter((line) => line.startsWith(prefix))
    .map((line) => JSON.parse(line.slice(prefix.length)));
}

if (bodyReference) {
  console.log(`Matched ${seeds.length * 9} post-placement body refinements with pinned Go.`);
}

function collectBodyStages() {
  const run = spawnSync(bodyReference, ['--default'], {
    ...options,
    env: { ...process.env, TALA_PILOT_TRACE_PLACEMENT_BODIES: '1' },
  });
  assert.ifError(run.error);
  assert.equal(run.status, 0, run.stderr);
  assert.equal(run.stdout, baseline, 'Body tracing changed the final pinned Go layout');
  return traceEntries(run.stderr, 'TALA_PILOT_PLACEMENT_BODIES ');
}

function checkBodyStages(seed, request, placed, routingBodies) {
  const history = new Map(
    bodyStages.filter((entry) => entry.seed === seed).map(({ index, nodes }) => [index, nodes])
  );
  const byAuthoredOrder = (nodes) => obstacleOrder.map((index) => nodes[index]);
  assert.deepEqual(history.get(7), byAuthoredOrder(placed), `NodePlacement, seed ${seed}`);
  for (const index of [13, 14, 18, 19, 21, 22, 23, 24]) {
    assert(history.has(index), `Missing Go stage ${index}, seed ${seed}`);
    assert.deepEqual(
      history.get(index),
      history.get(index - 1),
      `Go body coordinates changed at stage ${index}, seed ${seed}`
    );
  }
  const stages = new Map([
    [8, 'swapStuff'],
    [9, 'transpose'],
    [10, 'alignAxes'],
    [11, 'gapNormalization'],
    [12, 'postGapAlignAxes'],
    [15, 'balanceSymmetry'],
    [16, 'equidistance'],
    [17, 'finalAlignAxes'],
    [20, 'rescale'],
  ]);
  const actual = JSON.parse(
    execFileSync(rust, ['--flat-refinement-snapshots'], { ...options, input: request })
  ).map(({ stage, nodes }) => ({ stage, nodes: byAuthoredOrder(nodes) }));
  const expected = [...stages].map(([index, stage]) => ({ stage, nodes: history.get(index) }));
  assert.deepEqual(actual, expected, `Post-placement coordinates differ for seed ${seed}`);
  assert.deepEqual(history.get(20), byAuthoredOrder(routingBodies), `Routing bodies, seed ${seed}`);
}

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
