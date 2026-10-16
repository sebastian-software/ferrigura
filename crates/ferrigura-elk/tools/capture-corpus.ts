/* eslint-disable no-console -- CLI tool: progress and results go to the console */
/**
 * Differential ELK corpus generator for the ferrigura-elk port.
 *
 * 1. Renders the Mermaid e2e fixtures in scope in Chromium with ELK forced on
 *    (`layout: 'elk'`), capturing every graph Mermaid hands to elkjs. The page gets
 *    a `__mermaidElkCapture` callback before Mermaid loads; render.ts calls it with
 *    the input (copied before layout) and the result.
 * 2. Re-runs each captured input through Node elkjs (same bundle Mermaid ships),
 *    with `logging: true`, and compares the result with the browser result.
 * 3. Writes one case per captured call to crates/ferrigura-elk/tests/corpus/,
 *    plus the manifest and three hand-built nodes-only cases.
 *
 * Run from the repository root:
 *
 *   pnpm exec tsx crates/ferrigura-elk/tools/capture-corpus.ts
 *
 * With `--synthetic-only` it regenerates just the hand-built cases, without a browser.
 *
 * It starts `pnpm dev` on the port the e2e setup derives for this checkout, or
 * reuses a dev server that is already answering there, and stops it when done.
 */
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { basename, join, posix, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium, type Browser } from 'playwright';
import {
  CORPUS_VERSION,
  firstDifference,
  graphStats,
  jsonRoundTrip,
  nodesOnlyGraph,
  slugify,
  syntheticEdgeGraphs,
  syntheticNodeLabelGraphs,
  stripGwtHashKeys,
  type GraphStats,
  type Json,
} from './corpus-model.js';

const REPO_ROOT = resolve(fileURLToPath(new URL('.', import.meta.url)), '../../..');
const CORPUS_DIR = join(REPO_ROOT, 'crates/ferrigura-elk/tests/corpus');
const E2E_DIAGRAMS_DIR = join(REPO_ROOT, 'e2e/diagrams');

/** Fixture folders in scope, all under e2e/diagrams. */
const SCOPE_DIAGRAMS = [
  'flowchart',
  'class-diagram',
  'state-diagram',
  'state-diagram-v2',
  'er-diagram',
  'requirement',
];
/** Every fixture below a folder with this name is included. */
const PR_CHECK_FOLDER = 'pr-check';
/** Other in-scope fixtures are included only when their text already asks for ELK. */
const ELK_LAYOUT_PATTERN = /\blayout:\s*["']?elk\b/;
const SYNTHETIC_NODE_COUNTS = [1, 3, 5];

const RENDER_TIMEOUT_MS = 30_000;
const SERVER_BOOT_TIMEOUT_MS = 300_000;
const VIEWPORT = { width: 1440, height: 1024 };
/** Indentation of the written JSON. Change to 0 to shrink the corpus. */
const JSON_INDENT = 1;

interface Fixture {
  diagram: string;
  relPath: string;
  stem: string;
  text: string;
  inPrCheck: boolean;
}

/** Version of the capture entry shape; bump it when `CapturedCall` changes. */
const CAPTURE_VERSION = 1;

/** One entry of `globalThis.mermaidElkCaptures`, written by the page's capture callback. */
interface CapturedCall {
  version: number;
  index: number;
  algorithm: string | null;
  input: Json;
  output: Json;
}

type RenderOutcome =
  | { status: 'captured'; calls: CapturedCall[] }
  | { status: 'skipped'; category: string; detail: string };

interface NodeRun {
  output: Json;
  trace: unknown;
}

interface CaseRecord {
  name: string;
  fixture: string | null;
  algorithm: string | null;
  input: Json;
  output: Json;
  trace: unknown;
  stats: GraphStats;
}

type ElkConstructor = new () => {
  layout(graph: unknown, options?: { logging?: boolean }): Promise<Record<string, unknown>>;
};

const sleep = (ms: number) => new Promise((done) => setTimeout(done, ms));

function byName(a: { name: string }, b: { name: string }): number {
  return a.name < b.name ? -1 : a.name > b.name ? 1 : 0;
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixture selection
// ─────────────────────────────────────────────────────────────────────────────

function findMmdFiles(dir: string): string[] {
  const files: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true }).sort(byName)) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...findMmdFiles(full));
    } else if (entry.isFile() && entry.name.endsWith('.mmd')) {
      files.push(full);
    }
  }
  return files;
}

function selectFixtures(): Fixture[] {
  const selected: Fixture[] = [];
  for (const diagram of SCOPE_DIAGRAMS) {
    for (const file of findMmdFiles(join(E2E_DIAGRAMS_DIR, diagram))) {
      const relPath = posix.join(...relative(REPO_ROOT, file).split(sep));
      const text = readFileSync(file, 'utf8');
      const inPrCheck = relPath.split('/').includes(PR_CHECK_FOLDER);
      if (inPrCheck || ELK_LAYOUT_PATTERN.test(text)) {
        selected.push({ diagram, relPath, stem: basename(file, '.mmd'), text, inPrCheck });
      }
    }
  }
  return selected;
}

// ─────────────────────────────────────────────────────────────────────────────
// Dev server (mirrors the port logic of .esbuild/server.ts and playwright.config.ts)
// ─────────────────────────────────────────────────────────────────────────────

function devPortFor(root: string): number {
  const override = Number.parseInt(process.env.MERMAID_DEV_PORT ?? '', 10);
  if (Number.isFinite(override) && override > 0 && override < 65536) {
    return override;
  }
  if (basename(root) === 'alana-mermaid') {
    return 9000;
  }
  return 9000 + (createHash('sha1').update(root).digest().readUInt16BE(0) % 100);
}

async function isServing(baseUrl: string): Promise<boolean> {
  try {
    const response = await fetch(`${baseUrl}/e2e.html`, { signal: AbortSignal.timeout(2000) });
    return response.ok;
  } catch {
    return false;
  }
}

interface DevServer {
  baseUrl: string;
  stop(): void;
}

async function ensureDevServer(): Promise<DevServer> {
  const port = devPortFor(REPO_ROOT);
  const baseUrl = `http://localhost:${port}`;
  if (await isServing(baseUrl)) {
    console.log(`Reusing the dev server already running at ${baseUrl}`);
    return { baseUrl, stop: () => undefined };
  }

  console.log(`Starting pnpm dev on ${baseUrl} (first build can take a minute)`);
  const child = spawn('pnpm', ['dev'], {
    cwd: REPO_ROOT,
    env: { ...process.env, MERMAID_DEV_PORT: String(port) },
    detached: true,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let output = '';
  child.stdout.on('data', (chunk: Buffer) => (output += chunk.toString()));
  child.stderr.on('data', (chunk: Buffer) => (output += chunk.toString()));
  let exitCode: number | undefined;
  child.once('exit', (code) => (exitCode = code ?? -1));

  const stop = () => {
    if (exitCode === undefined && child.pid !== undefined) {
      process.kill(-child.pid, 'SIGTERM');
    }
  };

  // The server prints "Rebuild N Time (total)" once its first build has finished,
  // success or not. Wait for that line, then check that the build did not fail.
  const deadline = Date.now() + SERVER_BOOT_TIMEOUT_MS;
  while (!/Rebuild \d+ Time \(total\)/.test(output)) {
    if (exitCode !== undefined) {
      throw new Error(`pnpm dev exited with code ${exitCode}:\n${output.slice(-4000)}`);
    }
    if (Date.now() > deadline) {
      stop();
      throw new Error(
        `pnpm dev did not finish its first build within ${SERVER_BOOT_TIMEOUT_MS} ms`
      );
    }
    await sleep(250);
  }
  if (output.includes('Build failed')) {
    stop();
    throw new Error(`pnpm dev built with errors:\n${output.slice(-4000)}`);
  }
  if (!(await isServing(baseUrl))) {
    stop();
    throw new Error(`pnpm dev is not answering at ${baseUrl}/e2e.html`);
  }
  return { baseUrl, stop };
}

// ─────────────────────────────────────────────────────────────────────────────
// Browser capture
// ─────────────────────────────────────────────────────────────────────────────

/** Base64 of the e2e page payload, the same shape e2e/helpers/util.ts builds. */
function encodeGraph(code: string): string {
  return Buffer.from(JSON.stringify({ code, mermaid: { layout: 'elk' } }), 'utf8').toString(
    'base64'
  );
}

async function renderWithElkCapture(
  browser: Browser,
  baseUrl: string,
  fixture: Fixture
): Promise<RenderOutcome> {
  // A new page per fixture: the capture array starts empty and nothing leaks across fixtures.
  const page = await browser.newPage({ viewport: VIEWPORT });
  const pageErrors: string[] = [];
  page.on('pageerror', (error) => pageErrors.push(error.message));
  try {
    await page.addInitScript((version: number) => {
      // The hook render.ts calls with (input, output) after every elkjs layout.
      interface Capture {
        version: number;
        index: number;
        algorithm: string | null;
        input: unknown;
        output: unknown;
      }
      const page = globalThis as unknown as {
        mermaidElkCaptures?: Capture[];
        __mermaidElkCapture?: (input: unknown, output: unknown) => void;
      };
      page.mermaidElkCaptures = [];
      page.__mermaidElkCapture = (input: unknown, output: unknown) => {
        const captures = page.mermaidElkCaptures ?? [];
        const algorithm = (input as { layoutOptions?: Record<string, unknown> }).layoutOptions?.[
          'elk.algorithm'
        ];
        captures.push({
          version,
          index: captures.length,
          algorithm: typeof algorithm === 'string' ? algorithm : null,
          input,
          // Round-trip the result so the record holds plain JSON, as the input does.
          output: JSON.parse(JSON.stringify(output)),
        });
        page.mermaidElkCaptures = captures;
      };
    }, CAPTURE_VERSION);
    await page.goto(`${baseUrl}/e2e.html?graph=${encodeGraph(fixture.text)}`, {
      waitUntil: 'load',
    });
    try {
      await page.waitForFunction(
        () => (globalThis as unknown as { rendered?: boolean }).rendered === true,
        undefined,
        { timeout: RENDER_TIMEOUT_MS }
      );
    } catch {
      return {
        status: 'skipped',
        category: 'render timed out',
        detail: `no rendered flag within ${RENDER_TIMEOUT_MS} ms`,
      };
    }

    const roledescription = await page
      .locator('svg[aria-roledescription]')
      .first()
      .getAttribute('aria-roledescription', { timeout: 5000 })
      .catch(() => null);
    const calls = await page.evaluate(
      () =>
        (globalThis as unknown as { mermaidElkCaptures?: CapturedCall[] }).mermaidElkCaptures ?? []
    );

    if (pageErrors.length > 0) {
      return { status: 'skipped', category: 'page error', detail: pageErrors[0].slice(0, 300) };
    }
    if (roledescription === null) {
      return { status: 'skipped', category: 'no diagram svg', detail: 'page has no diagram' };
    }
    if (roledescription === 'error') {
      return {
        status: 'skipped',
        category: 'rendered as syntax-error diagram',
        detail: `aria-roledescription=${roledescription}`,
      };
    }
    if (calls.length === 0) {
      return {
        status: 'skipped',
        category: 'no ELK layout call',
        detail: 'front matter or the diagram type bypassed the ELK layout',
      };
    }
    return { status: 'captured', calls };
  } finally {
    await page.close();
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Node elkjs re-run
// ─────────────────────────────────────────────────────────────────────────────

const mermaidRequire = createRequire(join(REPO_ROOT, 'packages/mermaid/package.json'));
const ELK = mermaidRequire('elkjs/lib/elk.bundled.js') as ElkConstructor;
const ELKJS_VERSION = JSON.parse(readFileSync(mermaidRequire.resolve('elkjs/package.json'), 'utf8'))
  .version as string;

async function runNodeElk(input: Json): Promise<NodeRun> {
  // elkjs writes into the graph it is given (adds x/y/width/height and `logging`),
  // so it gets a copy and `input` keeps the exact pre-layout shape.
  const graph = jsonRoundTrip(input);
  const result = await new ELK().layout(graph, { logging: true });
  const { logging, ...laidOut } = result;
  return {
    output: stripGwtHashKeys(jsonRoundTrip(laidOut)) as Json,
    trace: stripGwtHashKeys(logging),
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Output
// ─────────────────────────────────────────────────────────────────────────────

function caseFileContent(record: CaseRecord): string {
  const body = {
    version: CORPUS_VERSION,
    name: record.name,
    fixture: record.fixture,
    algorithm: record.algorithm,
    input: record.input,
    output: record.output,
    trace: record.trace,
  };
  return `${JSON.stringify(body, null, JSON_INDENT)}\n`;
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

/**
 * `--synthetic-only`: regenerates only the hand-built cases (the nodes-only ones and the edge
 * graphs of `syntheticEdgeGraphs`) through Node elkjs, and replaces only their files and manifest
 * entries. The browser captures are left as they are.
 */
async function writeSyntheticOnly(): Promise<void> {
  const generated: CaseRecord[] = [];
  const add = async (name: string, graph: Record<string, unknown>): Promise<void> => {
    const input = jsonRoundTrip(graph) as Json;
    const run = await runNodeElk(input);
    generated.push({
      name,
      fixture: null,
      algorithm: 'elk.layered',
      input,
      output: run.output,
      trace: run.trace,
      stats: graphStats(input as Record<string, unknown>),
    });
  };
  for (const count of SYNTHETIC_NODE_COUNTS) {
    await add(`synthetic__nodes-only-${count}`, nodesOnlyGraph(count));
  }
  for (const { name, graph } of syntheticEdgeGraphs()) {
    await add(name, graph);
  }
  for (const { name, graph } of syntheticNodeLabelGraphs()) {
    await add(name, graph);
  }
  generated.sort(byName);
  for (const record of generated) {
    writeFileSync(join(CORPUS_DIR, `${record.name}.json`), caseFileContent(record));
  }
  const manifestPath = join(CORPUS_DIR, 'manifest.json');
  const existing = JSON.parse(readFileSync(manifestPath, 'utf8')) as {
    version: number;
    cases: { name: string }[];
  };
  const kept = existing.cases.filter((entry) => !entry.name.startsWith('synthetic__'));
  const cases = [
    ...kept,
    ...generated.map((record) => ({
      name: record.name,
      fixture: record.fixture,
      nodes: record.stats.nodes,
      edges: record.stats.edges,
      compound: record.stats.compound,
      edgeLabels: record.stats.edgeLabels,
    })),
  ].sort(byName);
  writeFileSync(
    manifestPath,
    `${JSON.stringify({ version: existing.version, cases }, null, JSON_INDENT)}\n`
  );
  console.log(
    `Wrote ${generated.length} synthetic cases: ${generated.map((r) => r.name).join(', ')}`
  );
}

async function main(): Promise<void> {
  console.log(`elkjs ${ELKJS_VERSION} (${mermaidRequire.resolve('elkjs/lib/elk.bundled.js')})`);
  if (process.argv.includes('--synthetic-only')) {
    await writeSyntheticOnly();
    return;
  }

  const fixtures = selectFixtures();
  const prCheckCount = fixtures.filter((fixture) => fixture.inPrCheck).length;
  console.log(
    `Selected ${fixtures.length} fixtures (${prCheckCount} from pr-check, ${fixtures.length - prCheckCount} with layout: elk)`
  );

  const records: CaseRecord[] = [];
  const skipped: { fixture: string; category: string; detail: string }[] = [];
  const nodeErrors: { name: string; message: string }[] = [];
  const mismatches: { name: string; difference: string }[] = [];
  let matched = 0;

  const server = await ensureDevServer();
  const browser = await chromium.launch({ headless: true });
  try {
    for (const [i, fixture] of fixtures.entries()) {
      const outcome = await renderWithElkCapture(browser, server.baseUrl, fixture);
      if (outcome.status === 'skipped') {
        skipped.push({ fixture: fixture.relPath, ...outcome });
        console.log(`[${i + 1}/${fixtures.length}] skip ${fixture.relPath}: ${outcome.category}`);
        continue;
      }
      for (const call of outcome.calls) {
        const name = `${fixture.diagram}__${slugify(fixture.stem)}__${call.index}`;
        let run: NodeRun;
        try {
          run = await runNodeElk(call.input);
        } catch (error) {
          nodeErrors.push({ name, message: String(error) });
          continue;
        }
        const difference = firstDifference(stripGwtHashKeys(call.output), run.output);
        if (difference === undefined) {
          matched++;
        } else {
          mismatches.push({ name, difference });
        }
        records.push({
          name,
          fixture: fixture.relPath,
          algorithm: call.algorithm,
          input: call.input,
          output: run.output,
          trace: run.trace,
          stats: graphStats(call.input as Record<string, unknown>),
        });
      }
      console.log(
        `[${i + 1}/${fixtures.length}] ok   ${fixture.relPath} (${outcome.calls.length} layout call${outcome.calls.length === 1 ? '' : 's'})`
      );
    }
  } finally {
    await browser.close();
    server.stop();
  }

  for (const count of SYNTHETIC_NODE_COUNTS) {
    const input = jsonRoundTrip(nodesOnlyGraph(count)) as Json;
    const run = await runNodeElk(input);
    records.push({
      name: `synthetic__nodes-only-${count}`,
      fixture: null,
      algorithm: 'elk.layered',
      input,
      output: run.output,
      trace: run.trace,
      stats: graphStats(input as Record<string, unknown>),
    });
  }

  records.sort(byName);
  const names = new Set(records.map((record) => record.name));
  if (names.size !== records.length) {
    throw new Error('Duplicate case names; two captures share a fixture stem and index');
  }

  mkdirSync(CORPUS_DIR, { recursive: true });
  for (const file of readdirSync(CORPUS_DIR)) {
    if (file.endsWith('.json')) {
      rmSync(join(CORPUS_DIR, file));
    }
  }
  for (const record of records) {
    writeFileSync(join(CORPUS_DIR, `${record.name}.json`), caseFileContent(record));
  }
  const manifest = {
    version: CORPUS_VERSION,
    cases: records.map((record) => ({
      name: record.name,
      fixture: record.fixture,
      nodes: record.stats.nodes,
      edges: record.stats.edges,
      compound: record.stats.compound,
      edgeLabels: record.stats.edgeLabels,
    })),
  };
  writeFileSync(
    join(CORPUS_DIR, 'manifest.json'),
    `${JSON.stringify(manifest, null, JSON_INDENT)}\n`
  );

  const totalBytes = readdirSync(CORPUS_DIR)
    .filter((file) => file.endsWith('.json'))
    .reduce((sum, file) => sum + statSync(join(CORPUS_DIR, file)).size, 0);

  console.log('');
  console.log(
    `Cases written: ${records.length} (+ manifest.json) to ${relative(REPO_ROOT, CORPUS_DIR)}`
  );
  console.log(`Total size: ${(totalBytes / 1_000_000).toFixed(2)} MB`);
  console.log(
    `Node vs browser: ${matched}/${records.length - SYNTHETIC_NODE_COUNTS.length} captured cases match`
  );
  for (const mismatch of mismatches) {
    console.log(`  mismatch ${mismatch.name}: ${mismatch.difference}`);
  }
  console.log(`Node elkjs errors: ${nodeErrors.length}`);
  for (const error of nodeErrors) {
    console.log(`  ${error.name}: ${error.message}`);
  }
  console.log(`Fixtures skipped: ${skipped.length}`);
  const byCategory = new Map<string, string[]>();
  for (const entry of skipped) {
    byCategory.set(entry.category, [...(byCategory.get(entry.category) ?? []), entry.fixture]);
  }
  for (const [category, fixtureList] of [...byCategory].sort(([a], [b]) => (a < b ? -1 : 1))) {
    console.log(`  ${category}: ${fixtureList.length}`);
    for (const fixture of fixtureList) {
      console.log(`    ${fixture}`);
    }
  }
}

main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
