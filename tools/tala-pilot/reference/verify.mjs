// Replay the captured Go fixtures and check the geometry without reading SVGs.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFile, readdir } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const [binaryPath, reportPath] = process.argv.slice(2);
assert(binaryPath && reportPath, 'Usage: node verify.mjs <go-binary> <comparison-output-dir>');
const binary = resolve(binaryPath);
const report = resolve(reportPath);
const inputSuffix = '.measured.json';
const cases = (await readdir(report))
  .filter((name) => name.endsWith(inputSuffix))
  .map((name) => name.slice(0, -inputSuffix.length))
  .sort();
assert(cases.length > 0, 'No measured fixtures found');

const epsilon = 1e-6;
const near = (a, b) => Math.abs(a - b) <= epsilon;
const within = (value, lower, upper) => value >= lower - epsilon && value <= upper + epsilon;

for (const id of cases) {
  const input = await readFile(join(report, `${id}${inputSuffix}`));
  const saved = await readFile(join(report, `${id}.go.geometry.json`), 'utf8');
  const run = () => execFileSync(binary, { input, encoding: 'utf8' });
  assert.equal(run(), saved, `${id} Go replay drifted from the capture`);
  assert.equal(run(), saved, `${id} Go replay is not repeatable`);

  const graph = JSON.parse(input);
  const go = JSON.parse(saved);
  const elk = JSON.parse(await readFile(join(report, `${id}.elk.geometry.json`), 'utf8'));
  assert.equal(go.seed, 1, `${id} seed`);
  assert.deepEqual(
    go.nodes.map((node) => node.id),
    graph.nodes.map((node) => node.id)
  );
  assert.deepEqual(
    go.edges.map((edge) => edge.id),
    graph.edges.map((edge) => edge.id)
  );

  const nodes = new Map(go.nodes.map((node) => [node.id, node]));
  const elkNodes = new Map(elk.nodes.map((node) => [node.id, node]));
  for (const inputNode of graph.nodes) {
    const goNode = nodes.get(inputNode.id);
    const elkNode = elkNodes.get(inputNode.id);
    const name = `${id} node ${inputNode.id}`;
    assert([goNode.x, goNode.y, goNode.width, goNode.height].every(Number.isFinite), name);
    if (!inputNode.isGroup) {
      // TALA's prescale reserves (two parallel edges + one) × 40 px at
      // both Workshop endpoints, leaving the existing 152 px width intact.
      const expectedHeight =
        id === 'workshop-edges' && ['sensor', 'panel'].includes(inputNode.id)
          ? 120
          : inputNode.height;
      assert(
        near(goNode.width, inputNode.width) && near(goNode.height, expectedHeight),
        `${name} unexpected TALA leaf dimensions`
      );
      assert(
        near(elkNode.width, inputNode.width) && near(elkNode.height, inputNode.height),
        `${name} ELK changed measured leaf dimensions`
      );
    }
    if (inputNode.parentId) {
      const parent = nodes.get(inputNode.parentId);
      assert(
        within(goNode.x, parent.x, parent.x + parent.width) &&
          within(goNode.x + goNode.width, parent.x, parent.x + parent.width) &&
          within(goNode.y, parent.y, parent.y + parent.height) &&
          within(goNode.y + goNode.height, parent.y, parent.y + parent.height),
        `${name} is outside its TALA container`
      );
    }
  }

  for (const [index, edge] of graph.edges.entries()) {
    const route = go.edges[index];
    const name = `${id} edge ${edge.id}`;
    assert(route.points.length >= 2, `${name} has no route`);
    assert(onPerimeter(route.points[0], nodes.get(edge.source)), `${name} has a detached source`);
    assert(
      onPerimeter(route.points.at(-1), nodes.get(edge.target)),
      `${name} has a detached target`
    );
    for (let i = 1; i < route.points.length; i++) {
      const a = route.points[i - 1];
      const b = route.points[i];
      assert(near(a.x, b.x) !== near(a.y, b.y), `${name} has a diagonal or zero-length leg`);
    }
    if (edge.label) {
      assert(
        route.label && [route.label.x, route.label.y].every(Number.isFinite),
        `${name} has no valid label center`
      );
      assert.equal(route.labelWidth, Math.ceil(edge.labelWidth));
      assert.equal(route.labelHeight, Math.ceil(edge.labelHeight));
    }
  }
  process.stdout.write(`${id}: ${go.nodes.length} boxes and ${go.edges.length} routes verified\n`);
}

function onPerimeter(point, box) {
  const right = box.x + box.width;
  const bottom = box.y + box.height;
  return (
    within(point.x, box.x, right) &&
    within(point.y, box.y, bottom) &&
    (near(point.x, box.x) || near(point.x, right) || near(point.y, box.y) || near(point.y, bottom))
  );
}
