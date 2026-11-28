import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { parseArgs } from 'node:util';

const { values } = parseArgs({
  options: { reference: { type: 'string' }, rust: { type: 'string' } },
});
assert(values.reference, 'Pass --reference followed by the pinned Go bridge path.');
assert(values.rust, 'Pass --rust followed by the native Rust replay binary path.');
const source = new URL('../../crates/ferrigura-tala/src/', import.meta.url);
const suites = [
  ['connected/pairs.json', 'input'],
  ['connected/chains.json', 'input'],
  ['connected/parallel.json', 'input'],
  ['hierarchy/fixtures.json', 'graph'],
  ['hierarchy/boundary-fixtures.json', 'graph'],
];

let cases = 0;
for (const [path, input] of suites) {
  const fixtures = JSON.parse(await readFile(new URL(path, source), 'utf8'));
  for (const fixture of fixtures) {
    const graph = JSON.stringify(fixture[input]);
    const go = capture(resolve(values.reference), [], graph);
    const rust = capture(resolve(values.rust), ['--layout'], graph);
    assert.deepEqual(
      geometry(go),
      geometry(fixture.expected),
      `${path}: ${fixture.name} stored reference`
    );
    assert.deepEqual(geometry(rust), geometry(go), `${path}: ${fixture.name} native layout`);
    cases++;
  }
}
console.log(`Verified ${cases} native cases against the pinned Go reference.`);

function capture(binary, args, input) {
  return JSON.parse(execFileSync(binary, args, { input, encoding: 'utf8' }));
}

function geometry(layout) {
  return {
    nodes: layout.nodes,
    edges: layout.edges.map(({ id, points, label }) => ({ id, points, label: label ?? null })),
  };
}
