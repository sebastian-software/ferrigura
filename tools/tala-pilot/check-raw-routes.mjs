// Compare native routing against observational traces from the pinned Go build.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

import { checkFinish } from './check-observatory-finish.mjs';

const [reference, tracedReference, rust, labelReference, ferrigura] = process.argv
  .slice(2)
  .map((path) => resolve(path));
assert(
  reference && tracedReference && rust,
  'usage: node check-raw-routes.mjs <Go> <traced Go> <Rust> [label-traced Go and Ferrigura]'
);
assert.equal(
  Boolean(labelReference),
  Boolean(ferrigura),
  'The full seed check requires both binaries.'
);

const input = await readFile(
  new URL('./captures/observatory-services.measured.json', import.meta.url),
  'utf8'
);
const options = { input, encoding: 'utf8', maxBuffer: 8_000_000, timeout: 30_000 };
const baseline = execFileSync(reference, [], options);
const trace = spawnSync(tracedReference, [], {
  ...options,
  env: { ...process.env, TALA_PILOT_TRACE_RAW_ROUTES: '1' },
});
assert.ifError(trace.error);
assert.equal(trace.status, 0, trace.stderr);
assert.equal(trace.stdout, baseline, 'Routing tracing changed the final pinned Go layout');

const records = (prefix) =>
  trace.stderr
    .split('\n')
    .filter((line) => line.startsWith(prefix))
    .map((line) => JSON.parse(line.slice(prefix.length)));
const captures = records('TALA_PILOT_RAW_ROUTES ');
assert.equal(captures.length, 3, 'Expected one ordinary routing capture for each Go flavor');
const routingInput = captures[0].input;
for (const capture of captures) {
  assert.deepEqual(capture.input, routingInput, 'Go flavor inputs differed');
}
const order = ['ShortestToLongest', 'LongestToShortest', 'Default'];
const expectedFlavors = order.map((flavor) => {
  const capture = captures.find((entry) => entry.flavor === flavor);
  assert(capture, `Go routing did not record ${flavor}`);
  assert.equal(capture.routes.length, 9, `Unexpected ${flavor} route count`);
  return { flavor, cost: capture.cost, routes: capture.routes };
});
const selected = records('TALA_PILOT_SELECTED_ROUTES ');
assert.equal(selected.length, 1, 'Expected one Go selected-route capture');
assert.equal(selected[0].routes.length, 9, 'Unexpected Go selected route count');

// The standalone Go bridge's single-seed mode uses seed 1. Build the
// corresponding routing input entirely from measured Ferrigura geometry.
const graph = JSON.parse(input);
const translated = runRust('--translate', graph);
const placed = runRust('--measured-flat-refinement', { graph, seed: 1 });
const members = new Map(translated.nodes.map((node) => [graph.nodes[node.inputIndex].id, node]));
const occupied = new Map(placed.map(({ id }, index) => [id, index]));
const routeOwner = (index) => occupied.get(graph.nodes[translated.nodes[index].inputIndex].id);
const derivedInput = {
  nodes: placed.map(({ id, ...body }) => ({ ...body, id: members.get(id).absoluteId })),
  nodeIds: placed.map(({ id }) => String(members.get(id).id)),
  // Measured ordinary rectangles keep their initial captions inside their
  // bodies, so they have no separate obstacle rectangle for routing.
  labels: placed.map(() => null),
  edges: translated.edges.map((edge) => ({
    owners: [routeOwner(edge.from), routeOwner(edge.to)],
    arrowheads: [edge.sourceArrow, edge.targetArrow].map((arrow) => (arrow ? 'triangle' : 'none')),
    appearance: {
      opacity: null,
      stroke: null,
      strokeWidth: null,
      strokeDash: null,
      animated: null,
    },
    invisible: false,
  })),
};
assert.deepEqual(derivedInput, routingInput, 'Native placement and route input differ from Go');
const native = runRust('--raw-routes', derivedInput);
assert.deepEqual(native.flavors, expectedFlavors, 'Native flavor routing differs from Go');
assert.deepEqual(native.selected, selected[0], 'Native selected route endpoints differ from Go');
const balance = records('TALA_PILOT_ROUTE_BALANCE ');
assert.equal(balance.length, 2, 'Expected a pin capture before and after edge balance');
assert.deepEqual(
  balance.map((capture) => capture.stage),
  ['before', 'after']
);
assert.deepEqual(native.before_balance, balance[0].routes, 'Go changed routes before balance');
assert.deepEqual(native.balanced, balance[1].routes, 'Native balanced routes differ from Go');
const moved = balance[1].routes.filter(
  (route, index) => JSON.stringify(route) !== JSON.stringify(balance[0].routes[index])
);
assert.equal(moved.length, 8, 'Unexpected number of balanced Go routes');
console.log(
  'Matched native measured placement, all 27 raw routes and costs, the chosen Go flavor, and all 9 materialized and balanced paths.'
);

if (labelReference) {
  await checkFinish({
    baseline,
    ferrigura,
    graph,
    labelReference,
    native,
    options,
    placed,
    runRust,
    translated,
  });
}

function runRust(command, input) {
  return JSON.parse(execFileSync(rust, [command], { ...options, input: JSON.stringify(input) }));
}
