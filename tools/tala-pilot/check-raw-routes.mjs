// Compare native routing against observational traces from the pinned Go build.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const [reference, tracedReference, rust] = process.argv.slice(2).map((path) => resolve(path));
assert(
  reference && tracedReference && rust,
  'usage: node check-raw-routes.mjs <Go> <traced Go> <Rust>'
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

const native = JSON.parse(
  execFileSync(rust, ['--raw-routes'], { ...options, input: JSON.stringify(routingInput) })
);
assert.deepEqual(native.flavors, expectedFlavors, 'Native flavor routing differs from Go');
assert.deepEqual(native.selected, selected[0], 'Native selected route endpoints differ from Go');
console.log(
  'Matched all 27 raw routes and costs, the chosen Go flavor, and its 9 materialized paths.'
);
