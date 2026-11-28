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

const edgePrefix = 'TALA_PILOT_OVG_EDGE_POINTS ';
const edgeCaptures = trace.stderr
  .split('\n')
  .filter((line) => line.startsWith(edgePrefix))
  .map((line) => JSON.parse(line.slice(edgePrefix.length)));
assert.equal(edgeCaptures.length, 1, `${name}: expected one edge-fill capture`);
const [edgeCapture] = edgeCaptures;
const { edges, addedPoints } = JSON.parse(
  await readFile(
    new URL(fixturePath.replace('vertices.json', 'edge-points.json'), import.meta.url),
    'utf8'
  )
);
const edgeExpected = {
  ...expected,
  vertices: [
    ...expected.vertices,
    ...addedPoints.map(([x, y]) => ({ point: { x, y }, owners: [] })),
  ],
};
assert.deepEqual(
  edgeCapture,
  { nodes, edges, expected: edgeExpected },
  `${name}: stored edge capture changed`
);
const nativeEdges = execFileSync(rust, ['--ovg-edge-points'], {
  ...options,
  input: JSON.stringify({ nodes, edges }),
});
assert.deepEqual(
  JSON.parse(nativeEdges),
  edgeExpected,
  `${name}: visibility graph after edge fill differs`
);
console.log(
  `Also matched ${addedPoints.length} ordered native perimeter and halfway vertices with pinned Go.`
);

const boundaryPrefix = 'TALA_PILOT_OVG_BOUNDARY_POINTS ';
const boundaryCaptures = trace.stderr
  .split('\n')
  .filter((line) => line.startsWith(boundaryPrefix))
  .map((line) => JSON.parse(line.slice(boundaryPrefix.length)));
assert.equal(boundaryCaptures.length, 1, `${name}: expected one boundary-point capture`);
const [boundaryCapture] = boundaryCaptures;
assert.deepEqual(boundaryCapture.nodes, nodes, `${name}: boundary node order changed`);
assert.deepEqual(boundaryCapture.edges, edges, `${name}: boundary edge order changed`);
const boundaryPoints = JSON.parse(
  await readFile(
    new URL(fixturePath.replace('vertices.json', 'boundary-points.json'), import.meta.url),
    'utf8'
  )
);
const nativeBoundary = execFileSync(rust, ['--ovg-boundary-points'], {
  ...options,
  input: JSON.stringify({ nodes, edges }),
});
for (const capture of [boundaryCapture.expected, JSON.parse(nativeBoundary)]) {
  const pointOffset = edgeExpected.vertices.length;
  assert.deepEqual(capture.portsByNode, edgeExpected.portsByNode);
  assert.deepEqual(capture.vertices.slice(0, pointOffset), edgeExpected.vertices);
  const points = capture.vertices.slice(pointOffset).map(({ owners, point: { x, y } }) => {
    assert.deepEqual(owners, []);
    return [x, y];
  });
  // Go ranges over its port-owner map, so this suffix has no defined order.
  points.sort(([ax, ay], [bx, by]) => ax - bx || ay - by);
  assert.deepEqual(points, boundaryPoints, `${name}: boundary coordinates differ`);
}
console.log(`Also matched all ${boundaryPoints.length} native boundary vertices with pinned Go.`);

const tunnelPrefix = 'TALA_PILOT_OVG_TUNNELS ';
const tunnelCaptures = trace.stderr
  .split('\n')
  .filter((line) => line.startsWith(tunnelPrefix))
  .map((line) => JSON.parse(line.slice(tunnelPrefix.length)));
assert.equal(tunnelCaptures.length, 1, `${name}: expected one tunnel capture`);
const [tunnelCapture] = tunnelCaptures;
assert.deepEqual(tunnelCapture.nodes, nodes, `${name}: tunnel node order changed`);
assert.deepEqual(tunnelCapture.edges, edges, `${name}: tunnel edge order changed`);
const tunnel = JSON.parse(
  await readFile(
    new URL(fixturePath.replace('vertices.json', 'tunnels.json'), import.meta.url),
    'utf8'
  )
);
const nativeTunnels = JSON.parse(
  execFileSync(rust, ['--ovg-tunnels'], {
    ...options,
    input: JSON.stringify({ nodes, edges }),
  })
);
const tunnelOffset = edgeExpected.vertices.length + boundaryPoints.length;
const canonical = (vertices) =>
  [...vertices].sort(({ point: a }, { point: b }) => a.x - b.x || a.y - b.y);
for (const capture of [tunnelCapture.expected, nativeTunnels]) {
  assert.deepEqual(capture.tunnelLinks, tunnel.links, `${name}: tunnel links differ`);
  assert.deepEqual(capture.vertices.slice(tunnelOffset), tunnel.vertices);
  assert.deepEqual(
    capture.portsByNode,
    edgeExpected.portsByNode.map((base, index) => [...base, ...tunnel.addedPorts[index]])
  );
}
assert.deepEqual(canonical(tunnelCapture.expected.vertices), canonical(nativeTunnels.vertices));
console.log(
  `Also matched ${tunnel.links.length} ordered native tunnel links and their port owners.`
);

const linksPrefix = 'TALA_PILOT_OVG_LINKS ';
const linkedCaptures = trace.stderr
  .split('\n')
  .filter((line) => line.startsWith(linksPrefix))
  .map((line) => JSON.parse(line.slice(linksPrefix.length)));
assert.equal(linkedCaptures.length, 1, `${name}: expected one linked visibility capture`);
const [linkedCapture] = linkedCaptures;
assert.deepEqual(linkedCapture.nodes, nodes, `${name}: linked node order changed`);
assert.deepEqual(linkedCapture.edges, edges, `${name}: linked edge order changed`);
const linkedFixture = JSON.parse(
  await readFile(
    new URL(fixturePath.replace('vertices.json', 'links.json'), import.meta.url),
    'utf8'
  )
);
const nativeLinked = JSON.parse(
  execFileSync(rust, ['--ovg-links'], {
    ...options,
    input: JSON.stringify({ nodes, edges }),
  })
);
const normalizeLinks = (graph) => {
  const centers = new Map(graph.centersByNode.map((vertex, owner) => [vertex, owner]));
  const ordered = Array.from(graph.vertices.keys()).sort((a, b) => {
    const first = graph.vertices[a].point;
    const second = graph.vertices[b].point;
    return (
      first.x - second.x || first.y - second.y || (centers.get(a) ?? -1) - (centers.get(b) ?? -1)
    );
  });
  const remap = new Map(ordered.map((original, index) => [original, index]));
  const mapIndex = (index) => remap.get(index);
  return {
    vertices: ordered.map((original) => graph.vertices[original]),
    centersByNode: graph.centersByNode.map(mapIndex),
    portsByNode: graph.portsByNode.map((ports) => ports.map(mapIndex)),
    links: graph.links.map((link) => link.map(mapIndex)),
    tunnelLinkCount: graph.tunnelLinkCount,
    nearPorts: ordered.flatMap((original, index) =>
      graph.nearPortOwners[original].map((owner) => [index, owner])
    ),
  };
};
const referenceLinked = normalizeLinks(linkedCapture.expected);
assert.equal(referenceLinked.vertices.length, linkedFixture.vertexCount);
assert.equal(referenceLinked.tunnelLinkCount, linkedFixture.tunnelLinkCount);
assert.deepEqual(referenceLinked.links, linkedFixture.links);
assert.deepEqual(referenceLinked.nearPorts, linkedFixture.nearPorts);
assert.deepEqual(
  normalizeLinks(nativeLinked),
  referenceLinked,
  `${name}: connected visibility graph differs`
);
console.log(
  `Also matched ${referenceLinked.links.length} ordered native visibility links and port proximity.`
);
