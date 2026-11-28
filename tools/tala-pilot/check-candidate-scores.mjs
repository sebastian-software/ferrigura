// Compare candidate scoring and seed selection with a pinned Go capture.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { parseArgs } from 'node:util';

const { values } = parseArgs({
  options: {
    reference: { type: 'string' },
    traced: { type: 'string' },
    rust: { type: 'string' },
    'write-fixtures': { type: 'string' },
  },
});
assert(values.reference && values.traced && values.rust, 'Pass --reference, --traced and --rust.');
const reference = resolve(values.reference);
const traced = resolve(values.traced);
const rust = resolve(values.rust);
const measured = new URL('captures/', import.meta.url);
const fixtures = (await readdir(measured)).filter((name) => name.endsWith('.measured.json')).sort();
assert(fixtures.length > 0);
const checkedIn = new Set(['library-architecture', 'observatory-services', 'publishing-flow']);
const snapshots = [];

for (const filename of fixtures) {
  const fixture = filename.slice(0, -'.measured.json'.length);
  const input = await readFile(new URL(filename, measured));
  const expected = run(reference, ['--default'], input);
  const observed = run(traced, ['--default'], input, { TALA_TRACE_CANDIDATE_SCORE: '1' });
  assert.equal(observed.stdout, expected.stdout, `${fixture}: tracing changed the oracle layout`);
  const lines = observed.stderr.trim().split('\n');
  const candidates = lines
    .filter((line) => line.startsWith('tala-candidate '))
    .map((line) => JSON.parse(line.slice('tala-candidate '.length)))
    .sort((a, b) => a.index - b.index);
  assert.deepEqual(
    candidates.map(({ seed }) => seed),
    [1, 2, 3],
    `${fixture}: ${observed.stderr}`
  );
  const selected = lines.filter((line) => line.startsWith('tala-selected '));
  assert.equal(selected.length, 1, `${fixture}: expected one recorded Go winner`);
  const selectedSeed = Number(selected[0].slice('tala-selected '.length));
  const evaluated = JSON.parse(run(rust, ['--candidate-score'], JSON.stringify(candidates)).stdout);
  const expectedEvaluations = candidates.map(({ penalty, area, work }) => ({
    score: { penalty, area },
    work,
  }));
  assert.deepEqual(evaluated.evaluations, expectedEvaluations, `${fixture}: exact scoring or work`);
  assert.equal(evaluated.selected_seed, selectedSeed, `${fixture}: winning seed`);
  if (checkedIn.has(fixture)) snapshots.push({ fixture, selected: selectedSeed, candidates });
  console.log(
    `${fixture}: ${candidates.length} candidate scores, work counts and seed ${selectedSeed} verified`
  );
}
if (values['write-fixtures']) {
  assert.equal(snapshots.length, checkedIn.size, 'Missing fixtures for the Rust stage test');
  await writeFile(values['write-fixtures'], `${JSON.stringify(snapshots, null, 2)}\n`);
}

function run(binary, args, input, env) {
  const processResult = spawnSync(binary, args, {
    encoding: 'utf8',
    input,
    env: { ...process.env, ...env },
    maxBuffer: 16 * 1024 * 1024,
    timeout: 60_000,
  });
  if (processResult.error) throw processResult.error;
  assert.equal(processResult.status, 0, `${binary}: ${processResult.stderr}`);
  return processResult;
}
