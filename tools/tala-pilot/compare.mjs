// Run with the compiled Ferrigura pilot example and the pinned Go capture runner.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { parseArgs } from 'node:util';

const cases = [
  [
    'library-architecture',
    'Library service architecture',
    'Nested groups and labeled links across group boundaries',
  ],
  ['publishing-flow', 'Publishing workflow', 'A long directed flow with two branches'],
  ['observatory-services', 'Observatory services', 'A cycle through a flat service network'],
  [
    'workshop-edges',
    'Workshop controls',
    'TALA grows the parallel edges’ endpoint nodes. The fixture also has a self-loop and disconnected components.',
  ],
  ['native-loop', 'Retry queue', 'An isolated rectangle with a labeled self-loop'],
  [
    'service-mesh',
    'Service mesh',
    'A denser graph with cycles and labeled edges across four groups',
  ],
];

const metrics = [
  ['crossings', 'Crossings'],
  ['bends', 'Bends'],
  ['length', 'Route length'],
  ['nodeOverlaps', 'Node overlaps'],
  ['labelOverlaps', 'Label overlaps'],
  ['resizedNodes', 'Resized nodes'],
  ['size', 'SVG dimensions'],
];

const { values } = parseArgs({
  options: {
    ferrigura: { type: 'string' },
    reference: { type: 'string' },
    captures: { type: 'string' },
    out: { type: 'string' },
  },
});
assert(
  values.ferrigura && Boolean(values.reference) !== Boolean(values.captures) && values.out,
  'Usage: node tools/tala-pilot/compare.mjs --ferrigura <pilot-binary> (--reference <go-binary> | --captures <dir>) --out <dir>'
);

const ferrigura = resolve(values.ferrigura);
const reference = values.reference && resolve(values.reference);
const captures = values.captures && resolve(values.captures);
const out = resolve(values.out);
assert.notEqual(out, captures, 'Use a separate output directory for capture replay');
await mkdir(out, { recursive: true });

const reports = [];
for (const [id, title, description] of cases) {
  const path = new URL(`fixtures/${id}.mmd`, import.meta.url);
  const input = await readFile(path, 'utf8');
  const sourcePath = join(out, `${id}.mmd`);
  await writeFile(sourcePath, input);
  const measured = run(ferrigura, ['measure', sourcePath]);
  await writeFile(join(out, `${id}.measured.json`), measured);
  const graph = JSON.parse(measured);
  const elk = run(ferrigura, ['elk-geometry', sourcePath]);
  let go;
  if (captures) {
    const capturedGraph = await readFile(join(captures, `${id}.measured.json`), 'utf8');
    assert.deepEqual(graph, JSON.parse(capturedGraph), `${id}: measured graph changed`);
    go = await readFile(join(captures, `${id}.go.geometry.json`), 'utf8');
  } else {
    go = run(reference, [], measured);
    assert.deepEqual(
      JSON.parse(run(reference, [], measured)),
      JSON.parse(go),
      `${id}: Go replay changed`
    );
  }

  const engines = [
    ['elk', 'ELK Layered', elk, run(ferrigura, ['elk', sourcePath])],
    ['go', 'TALA (Go reference)', go, await paint('go', go)],
  ];

  const results = [];
  for (const [engine, name, geometry, svg] of engines) {
    await writeFile(join(out, `${id}.${engine}.geometry.json`), geometry);
    await writeFile(join(out, `${id}.${engine}.svg`), svg);
    results.push({ engine, name, svg, ...measure(graph, JSON.parse(geometry), svg) });
  }

  reports.push({ id, title, description, input, results });

  async function paint(engine, geometry) {
    const geometryPath = join(out, `${id}.${engine}.geometry.json`);
    await writeFile(geometryPath, geometry);
    return run(ferrigura, ['paint', sourcePath, geometryPath]);
  }
}

const summary = reports.map(({ id, title, results }) => ({
  id,
  title,
  results: results.map(({ svg: _svg, ...values }) => values),
}));
await writeFile(join(out, 'metrics.json'), `${JSON.stringify(summary, null, 2)}\n`);
await writeFile(join(out, 'index.html'), document(reports, false));
const fragment = document(reports, true);
assert(Buffer.byteLength(fragment) < 1_000_000, 'The inline comparison must fit in 1 MB');
await writeFile(join(out, 'tala-vs-elk.html'), fragment);

function run(binary, args, input) {
  return execFileSync(binary, args, {
    encoding: 'utf8',
    input,
    timeout: 30_000,
    maxBuffer: 8_000_000,
  });
}

function measure(graph, layout, svg) {
  const epsilon = 1e-6;
  const byNodeId = new Map(graph.nodes.map((node) => [node.id, node]));
  const byEdgeId = new Map(graph.edges.map((edge) => [edge.id, edge]));
  assert.deepEqual(
    layout.nodes.map(({ id }) => id).sort(),
    [...byNodeId.keys()].sort(),
    'Layout must return every measured node exactly once'
  );
  assert.deepEqual(
    layout.edges.map(({ id }) => id).sort(),
    [...byEdgeId.keys()].sort(),
    'Layout must return every measured edge exactly once'
  );
  const leaves = layout.nodes.filter((node) => !byNodeId.get(node.id).isGroup);
  const labels = [];
  const routes = [];
  let length = 0;
  let bends = 0;
  for (const node of layout.nodes) {
    assert(
      [node.x, node.y, node.width, node.height].every(Number.isFinite),
      `Invalid bounds for ${node.id}`
    );
  }
  for (const edge of layout.edges) {
    const { labelWidth, labelHeight } = byEdgeId.get(edge.id);
    if (edge.label && labelWidth > 0 && labelHeight > 0) {
      assert(
        [edge.label.x, edge.label.y].every(Number.isFinite),
        `Invalid label bounds for ${edge.id}`
      );
      labels.push({
        x: edge.label.x - labelWidth / 2,
        y: edge.label.y - labelHeight / 2,
        width: labelWidth,
        height: labelHeight,
      });
    }
    const route = [];
    for (let index = 1; index < edge.points.length; index++) {
      const a = edge.points[index - 1];
      const b = edge.points[index];
      const vector = { x: b.x - a.x, y: b.y - a.y };
      const distance = Math.hypot(vector.x, vector.y);
      assert(Number.isFinite(distance), `Invalid route for ${edge.id}`);
      if (distance <= epsilon) continue;
      const previous = route.at(-1);
      length += distance;
      if (previous) {
        const sine = cross(previous.vector, vector) / (previous.distance * distance);
        const dot = previous.vector.x * vector.x + previous.vector.y * vector.y;
        if (Math.abs(sine) <= epsilon && dot > 0) {
          // Ignore redundant waypoints on a straight segment.
          previous.vector = { x: b.x - previous.a.x, y: b.y - previous.a.y };
          previous.distance = Math.hypot(previous.vector.x, previous.vector.y);
          continue;
        }
        bends++;
      }
      route.push({ a, vector, distance });
    }
    routes.push(route);
  }

  let crossings = 0;
  for (const [first, second] of pairs(routes)) {
    const intersections = new Set();
    for (const a of first) {
      for (const b of second) {
        const determinant = cross(a.vector, b.vector);
        if (Math.abs(determinant) <= epsilon * a.distance * b.distance) continue;
        const distance = { x: b.a.x - a.a.x, y: b.a.y - a.a.y };
        const ta = cross(distance, b.vector) / determinant;
        const tb = cross(distance, a.vector) / determinant;
        if (ta <= epsilon || ta >= 1 - epsilon || tb <= epsilon || tb >= 1 - epsilon) continue;
        const x = a.a.x + a.vector.x * ta;
        const y = a.a.y + a.vector.y * ta;
        intersections.add(`${x.toFixed(4)},${y.toFixed(4)}`);
      }
    }
    crossings += intersections.size;
  }

  const overlapping = (a, b) =>
    Math.min(a.x + a.width, b.x + b.width) - Math.max(a.x, b.x) > epsilon &&
    Math.min(a.y + a.height, b.y + b.height) - Math.max(a.y, b.y) > epsilon;
  const [, width, height] =
    /^<svg\b[^>]*\bviewBox="\S+\s+\S+\s+(\S+)\s+(\S+)"/.exec(svg.trim()) ?? [];
  assert(Number(width) > 0 && Number(height) > 0, 'SVG needs a positive viewBox');

  return {
    crossings,
    bends,
    length: Math.round(length),
    nodeOverlaps: [...pairs(leaves)].filter(([a, b]) => overlapping(a, b)).length,
    labelOverlaps:
      [...pairs(labels)].filter(([a, b]) => overlapping(a, b)).length +
      labels.reduce(
        (count, label) => count + leaves.filter((leaf) => overlapping(label, leaf)).length,
        0
      ),
    resizedNodes: leaves.filter((node) => {
      const measured = byNodeId.get(node.id);
      return (
        Math.abs(node.width - measured.width) > epsilon ||
        Math.abs(node.height - measured.height) > epsilon
      );
    }).length,
    width: Math.round(Number(width)),
    height: Math.round(Number(height)),
  };
}

function* pairs(values) {
  for (let i = 0; i < values.length; i++) {
    for (let j = i + 1; j < values.length; j++) yield [values[i], values[j]];
  }
}

function cross(a, b) {
  return a.x * b.y - a.y * b.x;
}

function escapeHtml(text) {
  return text
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}

function document(reports, inline) {
  const sections = reports.map(({ id, title, description, input, results }) => {
    const maxWidth = Math.max(...results.map(({ width }) => width));
    const figures = results.map((result) => {
      const filename = `${id}.${result.engine}.svg`;
      const src = inline
        ? `data:image/svg+xml;base64,${Buffer.from(result.svg).toString('base64')}`
        : filename;
      const img = `<img src="${src}" alt="${title} rendered with ${result.name}" style="width:${((result.width / maxWidth) * 100).toFixed(2)}%">`;
      return `<figure>
      <figcaption>${result.name}</figcaption>
      <div class="pilot-drawing">${inline ? img : `<a href="${filename}" aria-label="Open full-size ${result.name} SVG for ${title}">${img}</a>`}</div>
    </figure>`;
    });
    const rows = metrics.map(([key, label]) => {
      const cells = results.map((result) => {
        const value =
          key === 'size'
            ? `${result.width} × ${result.height}`
            : result[key].toLocaleString('en-US');
        return `<td>${value}</td>`;
      });
      return `<tr><th scope="row">${label}</th>${cells.join('')}</tr>`;
    });
    return `<section aria-labelledby="${id}">
  <h2 id="${id}">${title}</h2>
  <p class="text-muted">${description}</p>
  <div class="pilot-pair">${figures.join('\n')}</div>
  <table class="table table-sm" aria-label="Geometry metrics for ${title}">
    <thead><tr><th scope="col">Metric</th>${results.map(({ name }) => `<th scope="col">${name}</th>`).join('')}</tr></thead>
    <tbody>${rows.join('\n')}</tbody>
  </table>
  <details><summary>Mermaid source</summary><pre><code>${escapeHtml(input.trimEnd())}</code></pre></details>
</section>`;
  });

  const fragment = `<div id="ferrigura-tala-comparison">
<h1>ELK and TALA on the same Mermaid diagrams</h1>
<p class="pilot-method text-muted">Both columns use Ferrigura's text measurement and SVG renderer. The TALA column uses the pinned Go reference. The Rust normalization stage is verified separately.</p>
${sections.join('\n<hr>\n')}
${inline ? '' : '<p>Crossings count proper segment intersections between distinct edges. Bends count route turns. The overlap counts cover pairs of leaf nodes, pairs of edge labels, and edge labels against leaf nodes. Distances and dimensions are in SVG units. Resized nodes count leaf boxes whose output differs from the measured input.</p><p><a href="metrics.json">Read the measurements as JSON</a></p>'}
</div>
<style>
#ferrigura-tala-comparison section { display: grid; gap: .75em; }
#ferrigura-tala-comparison h1 { margin-bottom: .75em; }
#ferrigura-tala-comparison h2, #ferrigura-tala-comparison p { margin: 0; }
#ferrigura-tala-comparison .pilot-method { margin-bottom: 1.25em; }
#ferrigura-tala-comparison hr { margin-block: 1.25em; }
#ferrigura-tala-comparison .pilot-pair { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 15rem), 1fr)); gap: 1em; }
#ferrigura-tala-comparison figure { margin: 0; min-width: 0; }
#ferrigura-tala-comparison figcaption { margin-bottom: .5em; }
#ferrigura-tala-comparison .pilot-drawing { background: light-dark(var(--muted, #f5f6f7), var(--foreground, #eee)); padding: .6em; }
#ferrigura-tala-comparison img { display: block; height: auto; max-width: 100%; margin-inline: auto; }
#ferrigura-tala-comparison table { width: 100%; border-collapse: collapse; }
#ferrigura-tala-comparison th, #ferrigura-tala-comparison td { padding: .3em .5em; text-align: start; border-bottom: 1px solid var(--border, #ddd); overflow-wrap: anywhere; }
#ferrigura-tala-comparison td { font-variant-numeric: tabular-nums; }
#ferrigura-tala-comparison pre { white-space: pre-wrap; overflow-wrap: anywhere; }
</style>
`;
  if (inline) return fragment;

  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Ferrigura layout pilot · ELK and TALA</title>
<style>
body { max-width: 82rem; margin: auto; padding: 1.5rem; font: 1rem/1.5 system-ui, sans-serif; }
.text-muted { color: #555; }
</style>
</head>
<body>
<main>
${fragment}
</main>
</body>
</html>\n`;
}
