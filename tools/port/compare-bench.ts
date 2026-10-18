/**
 * The render times of `docs-port/comparisons/`: ferrigura through its Node addon, both as `render` and
 * through a `Renderer` that resolves the configuration once, Pele in Node and in Chromium, and Mermaid
 * in Chromium, on the `pr-check` fixtures of the types all three render. Each time is the median of
 * many renders of one fixture after a warm-up; a render that throws is null.
 *
 * Pele and Mermaid are installed outside the repository, so the lock file stays as it is, and the
 * addon is the release build:
 *
 * ```sh
 * npm install --prefix PACKAGES --ignore-scripts pele@1.6.0 mermaid@12.2.0
 * cargo build --release -p ferrigura-node
 * pnpm port:compare-bench PACKAGES OUT.json
 * ```
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';
import { extname, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { chromium } from 'playwright';

import { root } from './geometry/common.js';

/** The fixture folders of the types that ferrigura, Pele and Mermaid all render. */
const TYPES = [
  'flowchart',
  'sequence',
  'class-diagram',
  'state-diagram',
  'state-diagram-v2',
  'er-diagram',
  'gantt',
  'timeline',
  'git',
  'journey',
  'pie',
  'info',
  'mindmap',
  'block',
  'packet',
  'xychart',
  'sankey',
  'quadrant-chart',
  'requirement',
  'c4',
  'kanban',
  'architecture',
  'treemap',
  'ishikawa',
  'eventmodeling',
];

interface Fixture {
  type: string;
  file: string;
  text: string;
}

interface Result {
  type: string;
  file: string;
  ferrigura: number | null;
  ferriguraRenderer: number | null;
  peleNode: number | null;
  peleBrowser: number | null;
  mermaid: number | null;
}

type Render = (text: string) => unknown;

const [packagesArgument, outArgument] = process.argv.slice(2);
if (packagesArgument === undefined || outArgument === undefined) {
  throw new Error('usage: pnpm port:compare-bench PACKAGES OUT.json');
}
const packages = resolve(packagesArgument);

function fixtures(): Fixture[] {
  const found: Fixture[] = [];
  for (const type of TYPES) {
    const dir = join(root, 'e2e/diagrams', type, 'pr-check');
    if (!existsSync(dir)) continue;
    for (const file of readdirSync(dir)
      .filter((name) => name.endsWith('.mmd'))
      .sort()) {
      found.push({ type, file, text: readFileSync(join(dir, file), 'utf8') });
    }
  }
  return found;
}

function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  const mid = sorted.length >> 1;
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

/** The median of `runs` calls of `render` after three warm-up calls, or null when it throws. */
function timeSync(render: Render, text: string, runs: number): number | null {
  try {
    for (let i = 0; i < 3; i++) render(text);
    const times: number[] = [];
    for (let i = 0; i < runs; i++) {
      const start = performance.now();
      render(text);
      times.push(performance.now() - start);
    }
    return median(times);
  } catch {
    return null;
  }
}

/**
 * The addon, loaded as Node loads a `.node` file: its `render` with the defaults, and the `render` of a
 * `Renderer` made with the defaults, which resolves the configuration once.
 */
function ferriguraAddon(): { render: Render; renderer: Render } {
  const library =
    { darwin: 'libferrigura_node.dylib', win32: 'ferrigura_node.dll' }[
      process.platform as string
    ] ?? 'libferrigura_node.so';
  const addon = {
    exports: {} as {
      render: Render;
      Renderer: new () => { render: Render };
    },
  };
  process.dlopen(addon, join(root, 'target/release', library));
  const renderer = new addon.exports.Renderer();
  return {
    render: addon.exports.render,
    renderer: (text) => renderer.render(text),
  };
}

/** Serves the installed packages and an empty page, for the browser runs. */
async function serve(): Promise<{ url: string; close: () => void }> {
  const types: Record<string, string> = { '.mjs': 'text/javascript', '.js': 'text/javascript' };
  const server = createServer((request, response) => {
    const url = new URL(request.url ?? '/', 'http://localhost');
    if (url.pathname === '/') {
      response.writeHead(200, { 'content-type': 'text/html' });
      response.end('<!doctype html><html><body></body></html>');
      return;
    }
    const path = join(packages, 'node_modules', decodeURIComponent(url.pathname));
    if (!existsSync(path)) {
      response.writeHead(404);
      response.end();
      return;
    }
    response.writeHead(200, { 'content-type': types[extname(path)] ?? 'application/octet-stream' });
    response.end(readFileSync(path));
  });
  await new Promise<void>((done) => server.listen(0, done));
  const { port } = server.address() as AddressInfo;
  return { url: `http://localhost:${port}/`, close: () => server.close() };
}

/** The page side: Mermaid and Pele loaded from the server, with a timer for each. */
const PAGE_SETUP = `(async () => {
  const mermaid = (await import('/mermaid/dist/mermaid.esm.min.mjs')).default;
  mermaid.initialize({ startOnLoad: false });
  const pele = await import('/pele/dist/pele.js');
  const median = (values) => {
    const sorted = [...values].sort((a, b) => a - b);
    const mid = sorted.length >> 1;
    return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
  };
  let n = 0;
  window.timeMermaid = async (text, runs) => {
    try {
      for (let i = 0; i < 2; i++) await mermaid.render('w' + n++, text);
      const times = [];
      for (let i = 0; i < runs; i++) {
        const start = performance.now();
        await mermaid.render('m' + n++, text);
        times.push(performance.now() - start);
      }
      return median(times);
    } catch {
      return null;
    }
  };
  window.timePele = (text, runs) => {
    try {
      for (let i = 0; i < 3; i++) pele.render(text);
      const times = [];
      for (let i = 0; i < runs; i++) {
        const start = performance.now();
        pele.render(text);
        times.push(performance.now() - start);
      }
      return median(times);
    } catch {
      return null;
    }
  };
})()`;

const all = fixtures();
const ferrigura = ferriguraAddon();
// Pele in Node has no canvas, so its built-in width table measures the labels.
const peleModule = (await import(
  pathToFileURL(join(packages, 'node_modules/pele/dist/pele.js')).href
)) as { render: Render };
const results: Result[] = all.map((fixture) => ({
  type: fixture.type,
  file: fixture.file,
  ferrigura: timeSync(ferrigura.render, fixture.text, 30),
  ferriguraRenderer: timeSync(ferrigura.renderer, fixture.text, 30),
  peleNode: timeSync(peleModule.render, fixture.text, 30),
  peleBrowser: null,
  mermaid: null,
}));

const server = await serve();
const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1200, height: 900 } });
  await page.goto(server.url);
  await page.evaluate(PAGE_SETUP);
  for (const [index, fixture] of all.entries()) {
    results[index].mermaid = await page.evaluate(
      `window.timeMermaid(${JSON.stringify(fixture.text)}, 10)`
    );
    results[index].peleBrowser = await page.evaluate(
      `window.timePele(${JSON.stringify(fixture.text)}, 30)`
    );
  }
  const report = { chromium: browser.version(), node: process.version, results };
  await writeFile(outArgument, JSON.stringify(report, null, 1));
  console.log(`${all.length} fixtures, Chromium ${report.chromium}, Node ${report.node}`);
} finally {
  await browser.close();
  server.close();
}
