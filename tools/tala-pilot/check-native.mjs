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
const graphSuites = [
  ['connected/pairs.json', 'input'],
  ['connected/chains.json', 'input'],
  ['connected/parallel.json', 'input'],
  ['hierarchy/fixtures.json', 'graph'],
  ['hierarchy/boundary-fixtures.json', 'graph'],
  ['hierarchy/sibling-fixtures.json', 'graph'],
];

let cases = 0;
for (const [path, input] of graphSuites) {
  for (const fixture of await fixtures(path)) {
    verify(path, fixture.name, fixture[input], fixture.expected);
  }
}

const packing = 'placement/fixtures.json';
for (const { name, graph, nodes } of await fixtures(packing)) {
  if (nodes) verify(packing, name, graph, { nodes, edges: [] });
}

for (const path of ['routing/rectangle-loops.json', 'routing/rectangle-loop-groups.json']) {
  for (const fixture of await fixtures(path)) {
    const { id, width, height } = fixture.node;
    const node = {
      id,
      width,
      height,
      shape: 'rect',
      isGroup: false,
      label: '',
      labelWidth: 0,
      labelHeight: 0,
    };
    const graph = { nodes: [node], edges: fixture.edges ?? [fixture.edge] };
    const expected = {
      nodes: [fixture.expected_node],
      edges: fixture.expected_edges ?? [fixture.expected_edge],
    };
    verify(path, fixture.name, graph, expected);
  }
}
console.log(`Verified ${cases} native cases against the pinned Go reference.`);

async function fixtures(path) {
  return JSON.parse(await readFile(new URL(path, source), 'utf8'));
}

function verify(path, name, graph, expected) {
  const input = JSON.stringify(graph);
  const go = geometry(capture(resolve(values.reference), [], input));
  const rust = geometry(capture(resolve(values.rust), ['--layout'], input));
  assert.deepEqual(go, geometry(expected), `${path}: ${name} stored reference`);
  assert.deepEqual(rust, go, `${path}: ${name} native layout`);
  cases++;
}

function capture(binary, args, input) {
  return JSON.parse(execFileSync(binary, args, { input, encoding: 'utf8' }));
}

function geometry(layout) {
  return {
    nodes: layout.nodes ?? [],
    edges: (layout.edges ?? []).map(({ id, points, label }) => ({
      id,
      points,
      label: label ?? null,
    })),
  };
}
