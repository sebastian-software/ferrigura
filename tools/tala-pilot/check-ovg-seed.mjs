// Compare the captured reference stage and its native replay without changing reference geometry.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const [reference, tracedReference, rust] = process.argv.slice(2).map((path) => resolve(path));
assert(
  reference && tracedReference && rust,
  'usage: node check-ovg-seed.mjs <Go> <traced Go> <Rust>'
);

const input = await readFile(
  new URL('./captures/observatory-services.measured.json', import.meta.url),
  'utf8'
);
const fixturePath = '../../crates/ferrigura-tala/src/ovg_seed/vertices.json';
const name = 'Observatory services measured graph at initial OVG routing';
const fixtures = JSON.parse(await readFile(new URL(fixturePath, import.meta.url), 'utf8'));
const stored = fixtures.find((fixture) => fixture.name === name);
assert(stored, `${name}: stored Go capture missing`);
const { nodes, expected } = stored;

const options = { input, encoding: 'utf8', maxBuffer: 8_000_000, timeout: 30_000 };
const baseline = execFileSync(reference, [], options);
const trace = spawnSync(tracedReference, [], {
  ...options,
  env: { ...process.env, TALA_PILOT_TRACE_OVG_SEED: '1' },
});
assert.ifError(trace.error);
assert.equal(trace.status, 0, trace.stderr);
assert.equal(trace.stdout, baseline, 'Tracing changed the final pinned Go layout');

const prefix = 'TALA_PILOT_OVG_SEED ';
const captures = trace.stderr
  .split('\n')
  .filter((line) => line.startsWith(prefix))
  .map((line) => JSON.parse(line.slice(prefix.length)));
assert.equal(captures.length, 1, `${name}: expected one ordinary rectangular routing graph`);
const [capture] = captures;
assert.deepEqual(capture, { nodes, expected }, `${name}: stored Go capture changed`);

const native = execFileSync(rust, ['--ovg-seed'], {
  ...options,
  input: JSON.stringify(capture.nodes),
});
assert.deepEqual(JSON.parse(native), capture.expected, `${name}: initial visibility graph differs`);
console.log(
  `Matched ${capture.expected.vertices.length} ordered native visibility vertices with pinned Go.`
);
