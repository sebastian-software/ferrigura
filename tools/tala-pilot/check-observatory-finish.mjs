// Verify the latter half of the Observatory fixture from native balanced paths.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

export async function checkFinish({
  baseline,
  ferrigura,
  graph,
  labelReference,
  native,
  options,
  placed,
  runRust,
  translated,
}) {
  const labelTrace = spawnSync(labelReference, [], {
    ...options,
    env: { ...process.env, TALA_PILOT_TRACE_EDGE_LABEL: '1' },
  });
  assert.ifError(labelTrace.error);
  assert.equal(labelTrace.status, 0, labelTrace.stderr);
  assert.equal(labelTrace.stdout, baseline, 'Label tracing changed the fixed-seed Go layout');
  const prefix = 'TALA_PILOT_EDGE_LABEL ';
  const records = labelTrace.stderr
    .split('\n')
    .filter((line) => line.startsWith(prefix))
    .map((line) => JSON.parse(line.slice(prefix.length)));
  assert.equal(records.length, graph.edges.length);
  const byId = new Map(placed.map((body) => [body.id, body]));
  const nodes = graph.nodes.map(({ id }) => byId.get(id));
  const others = nodes.map(({ x, y, width, height }) => ({
    body: { x, y, width, height },
    isContainer: false,
  }));
  const labels = graph.nodes
    .filter(({ label }) => label)
    .map((node) => {
      const body = byId.get(node.id);
      const width = Math.ceil(node.labelWidth);
      const height = Math.ceil(node.labelHeight);
      return {
        x: body.x + body.width / 2 - width / 2,
        y: body.y + body.height / 2 - height / 2,
        width,
        height,
      };
    });
  const paths = native.balanced.map(({ points }) => points.map(([x, y]) => ({ x, y })));
  const shared = runRust('--shared-label-obstacles', paths);
  const order = paths
    .map((path, edge) => ({ edge, straight: path.length === 2, length: pathLength(path) }))
    .sort((a, b) => Number(a.straight) - Number(b.straight) || a.length - b.length);
  const anchored = Array(graph.edges.length).fill(null);
  for (const [step, { edge }] of order.entries()) {
    const trace = records[step];
    assert.equal(trace.cluster, false);
    const { label } = translated.edges[edge];
    const scenario = {
      route: edge,
      routes: paths,
      hint: null,
      percentage: 0,
      width: label.width,
      height: label.height,
      ancestors: [],
      others,
      labels,
      shared,
      repairConflicts: null,
    };
    assert.deepEqual(scenario, trace.input, `Native label input differs at edge ${edge}`);
    const [nativeSelection] = runRust('--edge-label-placement', [scenario]);
    assert.deepEqual(
      nativeSelection,
      trace.expected,
      `Native label choice differs at edge ${edge}`
    );
    const { anchor, percentage, body } = nativeSelection.best;
    labels.push(body);
    anchored[edge] = { anchor, percentage, width: label.width, height: label.height };
  }
  const final = runRust('--finalize-routes', {
    nodes,
    edges: graph.edges.map(({ id }, edge) => ({
      id,
      points: paths[edge],
      label: anchored[edge],
      reversedForLayout: translated.edges[edge].reversedForLayout,
    })),
  });
  const go = JSON.parse(baseline);
  assert.deepEqual(final.nodes, go.nodes, 'Final native node boxes differ from Go');
  assert.deepEqual(
    final.edges,
    go.edges.map(({ id, points, label }) => ({ id, points, label })),
    'Final native routes and labels differ from Go'
  );
  await checkPaintedSvg(ferrigura, baseline, final);
  console.log('Matched the complete fixed-seed Observatory geometry and painted SVG with Go.');
}

function pathLength(route) {
  let length = 0;
  for (const [index, point] of route.slice(1).entries()) {
    const start = route[index];
    const dx = point.x - start.x;
    const dy = point.y - start.y;
    length += dx === 0 ? Math.abs(dy) : dy === 0 ? Math.abs(dx) : Math.sqrt(dx * dx + dy * dy);
  }
  return length;
}

async function checkPaintedSvg(ferrigura, baseline, native) {
  const directory = await mkdtemp(join(tmpdir(), 'ferrigura-tala-fixed-seed-'));
  try {
    const mmd = fileURLToPath(new URL('fixtures/observatory-services.mmd', import.meta.url));
    const paint = (path) => execFileSync(ferrigura, ['paint', mmd, path], { timeout: 30_000 });
    const goFile = join(directory, 'go.json');
    const rustFile = join(directory, 'rust.json');
    await writeFile(goFile, baseline);
    await writeFile(rustFile, JSON.stringify(native));
    assert.deepEqual(
      paint(rustFile),
      paint(goFile),
      'The fixed-seed native Observatory SVG differs'
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}
