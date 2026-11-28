// Compare the ordinary hierarchy input, ordered scopes and rebound edges.
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
assert(values.reference && values.trace && values.rust, 'Pass --reference, --trace and --rust.');

const captures = new URL('captures/', import.meta.url);
const librarySnapshot = new URL(
  '../../crates/ferrigura-tala/src/layout_graph/hierarchy_preprocess/library.json',
  import.meta.url
);
const treeSnapshot = new URL(
  '../../crates/ferrigura-tala/src/layout_graph/tree_preprocess/fixtures.json',
  import.meta.url
);
const files = (await readdir(captures)).filter((file) => file.endsWith('.measured.json')).sort();
assert.equal(files.length, 8, 'Expected all known measured fixtures.');
const processOptions = { encoding: 'utf8', maxBuffer: 8_000_000, timeout: 60_000 };
const prefix = 'tala-hierarchy-preprocess ';
const treeSnapshots = [];
let checked = 0;
let snapshot;

for (const file of files) {
  const name = file.replace(/\.measured\.json$/, '');
  const input = await readFile(new URL(file, captures), 'utf8');
  const options = { ...processOptions, input };
  const traced = spawnSync(resolve(values.trace), ['--default'], {
    ...options,
    env: { ...process.env, TALA_TRACE_HIERARCHY_PREPROCESS: '1' },
  });
  assert.ifError(traced.error);
  assert.equal(traced.status, 0, `${name}: ${traced.stderr}`);
  const original = execFileSync(resolve(values.reference), ['--default'], options);
  assert.equal(traced.stdout, original, `${name}: the trace changed the final Go geometry`);
  const decoded = traced.stderr
    .split('\n')
    .flatMap((line) => (line.startsWith(prefix) ? [JSON.parse(line.slice(prefix.length))] : []));
  const actual = JSON.parse(
    execFileSync(resolve(values.rust), ['--hierarchy-preprocess'], options)
  );
  const preprocessed = JSON.parse(
    execFileSync(resolve(values.rust), ['--tree-preprocess'], options)
  );
  for (const seed of [1, 2, 3]) {
    const checkpoints = decoded.filter((record) => record.seed === seed);
    assert.equal(checkpoints.length, 5, `${name}: expected all five early Go stages per seed`);
    const upstream = checkpoints.find(({ stage }) => stage === 'PreprocessTrees');
    const prescale = checkpoints.find(({ stage }) => stage === 'Prescale');
    assert(
      upstream?.scopes?.length && prescale?.scopes?.length,
      `${name}: missing hierarchy trace for seed ${seed}`
    );
    assert.deepEqual(
      actual,
      { candidates: prescale.candidates, scopes: prescale.scopes },
      `${name}: measured hierarchy projection differs before preprocessing for seed ${seed}`
    );
    const { candidates, scopes, treeState } = upstream;
    const expected = { candidates, scopes };
    assert(treeState, `${name}: missing tree state for seed ${seed}`);
    assert.deepEqual(
      preprocessed.tree,
      treeState,
      `${name}: tree preprocessing differs for seed ${seed}`
    );
    assert.deepEqual(
      preprocessed.hierarchy,
      expected,
      `${name}: hierarchy input differs for seed ${seed}`
    );
    checked += 1;
    if (name === 'library-architecture' && seed === 1) {
      snapshot = { input: JSON.parse(input), expected };
    }
    if (
      ['publishing-flow', 'workshop-edges', 'library-architecture'].includes(name) &&
      seed === 1
    ) {
      treeSnapshots.push({
        name,
        input: JSON.parse(input),
        expected: { tree: treeState, hierarchy: expected },
      });
    }
  }
}

assert.equal(checked, 24);
assert(snapshot, 'The Library Architecture snapshot was not captured.');
assert.equal(treeSnapshots.length, 3, 'Expected the three tree regression captures.');
if (values.write) {
  await writeFile(librarySnapshot, `${JSON.stringify(snapshot, null, 2)}\n`);
  await writeFile(treeSnapshot, `${JSON.stringify(treeSnapshots, null, 2)}\n`);
} else {
  assert.deepEqual(snapshot, JSON.parse(await readFile(librarySnapshot, 'utf8')));
  assert.deepEqual(treeSnapshots, JSON.parse(await readFile(treeSnapshot, 'utf8')));
}
console.log(
  `Verified ${checked} measured projections, tree states and hierarchy inputs. Publishing Flow pruned a tree and Workshop Edges restored one in a new order.`
);
