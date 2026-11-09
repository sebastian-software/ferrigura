/* eslint-disable security/detect-non-literal-fs-filename -- Paths are a fresh temporary directory and fixed files in this repository. */
// Draws the charts of the size page from app/data/size.json with the published
// @ferrigura/core and writes app/data/size-charts.json and the SVG files in
// app/data/size/. The page imports them; nobody edits them by hand.
//
//   node scripts/render-size.mjs          # write the files
//   node scripts/render-size.mjs --check  # fail when they drifted
//
// The numbers come from tools/size/measure.mjs, which builds the addon once
// per diagram type and takes minutes; run it, then this script, when a change
// moves the sizes. The charts are Mermaid text that Ferrigura draws, so the
// page shows its own output. Writing renders with the release the homepage's
// other drawings come from (app/data/diagram-samples.json); the check renders
// again with the recorded release.
import assert from 'node:assert/strict';
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { format } from 'oxfmt';

import { compareOutput, installPublished } from './published-output.mjs';

const check = process.argv.includes('--check');
const name = '@ferrigura/core';
const data = new URL('../app/data/', import.meta.url);
const target = new URL('size-charts.json', data);
const svgDirectory = new URL('size/', data);
const size = JSON.parse(await readFile(new URL('size.json', data), 'utf8'));
const recorded = check ? JSON.parse(await readFile(target, 'utf8')) : undefined;
const samples = JSON.parse(await readFile(new URL('diagram-samples.json', data), 'utf8'));
const version = recorded?.renderedWith[name] ?? samples.renderedWith[name];
const platform = `${process.platform}-${process.arch}`;

/** The names the diagram pages use, by the addon's feature names. */
const labels = {
  pie: 'Pie',
  flowchart: 'Flowchart',
  timeline: 'Timeline',
  journey: 'User journey',
  git_graph: 'Git graph',
  er: 'Entity relationship',
  class: 'Class',
  sequence: 'Sequence',
  state: 'State',
  gantt: 'Gantt',
  mindmap: 'Mindmap',
  block: 'Block',
  packet: 'Packet',
  xy_chart: 'XY chart',
  sankey: 'Sankey',
  quadrant_chart: 'Quadrant chart',
  requirement: 'Requirement',
  c4: 'C4',
  kanban: 'Kanban',
  architecture: 'Architecture',
  treemap: 'Treemap',
  ishikawa: 'Ishikawa',
  event_modeling: 'Event modeling',
  agentflow: 'Agent flow',
  cynefin: 'Cynefin',
  radar: 'Radar',
  railroad: 'Railroad',
  swimlanes: 'Swimlanes',
  tree_view: 'Tree view',
  use_case: 'Use case',
  venn: 'Venn',
  wardley: 'Wardley map',
};
for (const { type } of size.types) assert(labels[type], `No label for the diagram type ${type}`);

const kB = (bytes) => Math.round(bytes / 1000);
const MB = (bytes) => Math.round(bytes / 100_000) / 10;
/** Bars in the site's accent, on a chart tall enough for its categories. */
const chartConfig = (height) =>
  `%%{init: {"xyChart": {"height": ${height}, "width": 720}, "themeVariables": {"xyChart": {"plotColorPalette": "#b5512e"}}}}%%`;

const types = size.types.toSorted((a, b) => b.adds - a.adds);
const top = Math.ceil(kB(types[0].adds) / 100) * 100;
const charts = [
  {
    id: 'types',
    title: 'What each diagram type adds',
    description: `The addon with one diagram type, less the core (${MB(size.core.bytes)} MB), in kB. Types that share a layout engine each count it here; together they carry it once.`,
    source: `${chartConfig(40 + types.length * 26)}
xychart-beta horizontal
  x-axis [${types.map(({ type }) => `"${labels[type]}"`).join(', ')}]
  y-axis "kB added to the core" 0 --> ${top}
  bar [${types.map(({ adds }) => kB(adds)).join(', ')}]
`,
  },
  {
    id: 'sets',
    title: 'The addon by the types it draws',
    description:
      'Builds of the addon with the cargo features of a few type sets, in MB, stripped. The core is the build with the info diagram alone.',
    source: `${chartConfig(300)}
xychart-beta horizontal
  x-axis ["Core", ${size.sets.map(({ label }) => `"${label}"`).join(', ')}]
  y-axis "MB" 0 --> ${Math.ceil(MB(size.sets.at(-1).bytes) + 1)}
  bar [${[size.core, ...size.sets].map(({ bytes }) => MB(bytes)).join(', ')}]
`,
  },
  {
    id: 'share',
    title: 'Core and diagram types in the full addon',
    description:
      'The published addon, which draws every type, split into the core and what the 32 other types add together, in MB.',
    source: `pie showData
  "Core" : ${MB(size.core.bytes)}
  "Diagram types" : ${MB(size.sets.at(-1).bytes - size.core.bytes)}
`,
  },
];

/** A recorded file against a fresh render, exact on the recording platform. */
const compare = (options) =>
  compareOutput({ ...options, message: `${options.label} drifted; run pnpm size:write` });

const packages = await installPublished({ [name]: version }, { quiet: check });
try {
  const { Renderer } = await import(pathToFileURL(packages.require.resolve(name)).href);
  const renderer = new Renderer();
  const svgs = new Map();
  for (const chart of charts) {
    const svg = renderer.render(chart.source, { colorScheme: 'light', id: `size-${chart.id}` });
    assert.match(svg, /<text\b/, `${chart.id} has no SVG text`);
    assert.doesNotMatch(svg, /<foreignObject\b|<script\b/, `${chart.id} contains HTML or script`);
    svgs.set(`${chart.id}.svg`, svg);
    chart.file = `${chart.id}.svg`;
  }
  const exact = !check || recorded.platform === platform;
  const output = {
    renderedWith: { [name]: version },
    platform: check ? recorded.platform : platform,
    charts,
  };
  const formatted = await format('size-charts.json', JSON.stringify(output));
  assert.equal(formatted.errors.length, 0);
  if (check) {
    compare({
      actual: await readFile(target, 'utf8'),
      expected: formatted.code,
      label: 'size-charts.json',
      exact,
    });
    const entries = await readdir(svgDirectory);
    const committed = entries.filter((entry) => entry.endsWith('.svg'));
    assert.deepEqual(committed.toSorted(), [...svgs.keys()].toSorted(), 'The SVG files drifted');
    for (const [entry, content] of svgs)
      compare({
        actual: await readFile(new URL(entry, svgDirectory), 'utf8'),
        expected: content,
        label: entry,
        exact,
      });
    console.log(`Verified ${svgs.size} size charts against ${name} ${version}.`);
  } else {
    await rm(svgDirectory, { recursive: true, force: true });
    await mkdir(svgDirectory, { recursive: true });
    for (const [entry, content] of svgs) await writeFile(new URL(entry, svgDirectory), content);
    await writeFile(target, formatted.code);
    console.log(`Wrote ${svgs.size} size charts rendered by ${name} ${version}.`);
  }
} finally {
  await packages.remove();
}
