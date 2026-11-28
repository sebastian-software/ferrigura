// Compare translation and prescale with observational checkpoints from the pinned Go engine.
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
assert(
  values.reference && values.trace && values.rust,
  'Pass --reference, --trace and --rust with the Go, traced Go and Rust binaries.'
);

const captures = new URL('captures/', import.meta.url);
const snapshotPath = new URL(
  '../../crates/ferrigura-tala/src/layout_graph/fixtures.json',
  import.meta.url
);
const trackedCases = new Set(['library-architecture', 'workshop-edges']);
const measuredFiles = (await readdir(captures))
  .filter((file) => file.endsWith('.measured.json'))
  .sort();
assert(measuredFiles.length > 0, 'No measured flowchart inputs');
const options = { encoding: 'utf8', maxBuffer: 8_000_000, timeout: 60_000 };
const snapshots = [];
const prefix = 'TALA_PILOT_LAYOUT_GRAPH ';

for (const file of measuredFiles) {
  const name = file.replace(/\.measured\.json$/, '');
  const input = await readFile(new URL(file, captures), 'utf8');
  const plain = execFileSync(resolve(values.reference), ['--default'], { ...options, input });
  const trace = spawnSync(resolve(values.trace), ['--default'], {
    ...options,
    input,
    env: { ...process.env, TALA_PILOT_TRACE_LAYOUT_GRAPH: '1' },
  });
  assert.ifError(trace.error);
  assert.equal(trace.status, 0, `${name}: ${trace.stderr}`);
  assert.equal(trace.stdout, plain, `${name}: trace changed the final Go output`);
  const checkpoints = trace.stderr.split('\n').filter((line) => line.startsWith(prefix));
  assert.equal(checkpoints.length, 1, `${name}: expected exactly one translated input`);
  const { translated, prescaled } = JSON.parse(checkpoints[0].slice(prefix.length));
  for (const [command, expected] of [
    ['--translate', translated],
    ['--prescale', prescaled],
  ]) {
    const result = execFileSync(resolve(values.rust), [command], { ...options, input });
    assert.deepEqual(JSON.parse(result), expected, `${name}: ${command} differs from Go`);
  }
  if (trackedCases.has(name)) {
    snapshots.push({
      name,
      input: JSON.parse(input),
      translated,
      prescaledSizes: prescaled.nodes.map(({ width, height }) => [width, height]),
    });
  }
}

assert.equal(snapshots.length, trackedCases.size, 'Some checked-in fixtures have no Go capture');
if (values.write) {
  await writeFile(snapshotPath, JSON.stringify(snapshots, null, 2) + '\n');
} else {
  const stored = JSON.parse(await readFile(snapshotPath, 'utf8'));
  assert.deepEqual(snapshots, stored, 'Checked-in adapter fixtures differ from fresh Go');
}
console.log(`Matched Go translation and prescale on ${measuredFiles.length} measured flowcharts.`);
