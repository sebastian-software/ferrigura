// Compare candidate scoring and seed selection with a pinned Go capture.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

const { values } = parseArgs({
  options: {
    reference: { type: 'string' },
    traced: { type: 'string' },
    rust: { type: 'string' },
    ferrigura: { type: 'string' },
    'write-fixtures': { type: 'string' },
  },
});
assert(values.reference && values.traced && values.rust, 'Pass --reference, --traced and --rust.');
const reference = resolve(values.reference);
const traced = resolve(values.traced);
const rust = resolve(values.rust);
const ferrigura = values.ferrigura && resolve(values.ferrigura);
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
  if (ferrigura && fixture === 'observatory-services') {
    await checkNativeObservatory(input, expected.stdout, candidates, selectedSeed);
  }
  if (checkedIn.has(fixture)) snapshots.push({ fixture, selected: selectedSeed, candidates });
  console.log(
    `${fixture}: ${candidates.length} candidate scores, work counts and seed ${selectedSeed} verified`
  );
}
if (values['write-fixtures']) {
  assert.equal(snapshots.length, checkedIn.size, 'Missing fixtures for the Rust stage test');
  await writeFile(values['write-fixtures'], `${JSON.stringify(snapshots, null, 2)}\n`);
}

async function checkNativeObservatory(input, defaultCapture, candidates, selectedSeed) {
  const source = fileURLToPath(new URL('fixtures/observatory-services.mmd', import.meta.url));
  const measured = run(ferrigura, ['measure', source]).stdout;
  const graph = JSON.parse(measured);
  assert.deepEqual(graph, JSON.parse(input), 'Observatory measured fixture has changed');

  const scratch = await mkdtemp(join(tmpdir(), 'ferrigura-tala-seeds-'));
  try {
    for (const { seed, penalty, area, work } of candidates) {
      const expected = run(reference, ['--seed', String(seed)], measured).stdout;
      const candidate = JSON.parse(
        run(rust, ['--flat-layout'], JSON.stringify({ graph, seed })).stdout
      );
      assert.equal(candidate.seed, seed, 'native seed');
      assert.deepEqual(candidate.score, { penalty, area }, `seed ${seed}: exact score`);
      assert.equal(candidate.work, work, `seed ${seed}: exact scoring work`);
      await checkLayout(expected, candidate.layout, `seed ${seed}`);
    }

    const winner = JSON.parse(run(rust, ['--flat-layout-default'], measured).stdout);
    assert.equal(winner.seed, selectedSeed, 'native winning seed');
    await checkLayout(defaultCapture, winner.layout, 'default layout');
    console.log(
      'Observatory: all three native seeds and the default winner match Go geometry and SVG.'
    );
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }

  async function checkLayout(expected, actual, label) {
    assert.deepEqual(geometry(actual), geometry(JSON.parse(expected)), `${label}: exact geometry`);
    const referencePath = join(scratch, 'reference.json');
    const nativePath = join(scratch, 'native.json');
    await writeFile(referencePath, expected);
    await writeFile(nativePath, JSON.stringify(actual));
    const expectedSvg = run(ferrigura, ['paint', source, referencePath]).stdout;
    const actualSvg = run(ferrigura, ['paint', source, nativePath]).stdout;
    assert.equal(actualSvg, expectedSvg, `${label}: byte-identical SVG`);
  }
}

function geometry(layout) {
  return {
    nodes: layout.nodes,
    edges: layout.edges.map(({ id, points, label }) => ({ id, points, label: label ?? null })),
  };
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
