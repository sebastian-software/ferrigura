// Compare label search choices and scores from the pinned Go pipeline.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFile, readdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const [reference, traced, rust, flag] = process.argv.slice(2);
assert(reference && traced && rust, 'Pass the Go, traced Go and Rust binaries.');
assert(!flag || flag === '--write-fixture', 'The only optional argument is --write-fixture.');
const directory = new URL('captures/', import.meta.url);
const files = (await readdir(directory)).filter((file) => file.endsWith('.measured.json')).sort();
assert(files.length > 0, 'No measured flowcharts');
const prefix = 'TALA_PILOT_EDGE_LABEL ';
const options = { encoding: 'utf8', maxBuffer: 16_000_000, timeout: 60_000 };
let compared = 0;
let clusterLabels = 0;
let repairs = 0;

for (const file of files) {
  const input = await readFile(new URL(file, directory), 'utf8');
  const baseline = execFileSync(resolve(reference), ['--default'], { ...options, input });
  const trace = spawnSync(resolve(traced), ['--default'], {
    ...options,
    input,
    env: { ...process.env, TALA_PILOT_TRACE_EDGE_LABEL: '1' },
  });
  assert.ifError(trace.error);
  assert.equal(trace.status, 0, `${file}: ${trace.stderr}`);
  assert.equal(trace.stdout, baseline, `${file}: tracing changed the final Go layout`);
  const observations = trace.stderr
    .split('\n')
    .filter((line) => line.startsWith(prefix))
    .map((line) => JSON.parse(line.slice(prefix.length)));
  const ordinary = observations.filter(({ cluster }) => !cluster);
  clusterLabels += observations.length - ordinary.length;
  if (file === 'observatory-services.measured.json') {
    assert.equal(ordinary.length, 27, 'Missing an ordinary Observatory edge label search');
  }
  if (file === 'library-architecture.measured.json') {
    const repair = ordinary
      .filter(({ input }) => input.repairConflicts !== null)
      .sort((left, right) => left.expected.best.body.y - right.expected.best.body.y)[0];
    assert(repair, 'Missing the Library repair search');
    const fixture = { input: repair.input, expected: repair.expected };
    const fixturePath = new URL(
      '../../crates/ferrigura-tala/src/labeling/library-repair.json',
      import.meta.url
    );
    if (flag) {
      await writeFile(fixturePath, JSON.stringify(fixture, null, 2) + '\n');
    } else {
      assert.deepEqual(fixture, JSON.parse(await readFile(fixturePath, 'utf8')));
    }
  }
  const output = execFileSync(resolve(rust), ['--edge-label-placement'], {
    ...options,
    input: JSON.stringify(ordinary.map(({ input }) => input)),
  });
  const selections = JSON.parse(output);
  assert.equal(selections.length, ordinary.length, `${file}: changed the number of label searches`);
  for (const [index, { expected }] of ordinary.entries()) {
    assert.deepEqual(selections[index], expected, `${file}: label search ${index}`);
  }
  compared += ordinary.length;
  repairs += ordinary.filter(({ input }) => input.repairConflicts !== null).length;
}
console.log(
  `Matched ${compared} ordinary edge-label searches including ${repairs} repair searches. Cluster-specific searches retained in the trace: ${clusterLabels}.`
);
