/* eslint-disable no-console -- CLI tool: progress and results go to the console */
/**
 * Differential cose-bilkent corpus generator for the ferrigura-cose port.
 *
 * 1. Renders the Mermaid mindmap fixtures under e2e/diagrams/mindmap/ and the generated mindmaps
 *    (generate-trees.ts) in Chromium. The page gets a `__mermaidCoseCapture` hook before Mermaid
 *    loads. layout.ts calls it after every cose-bilkent layout with the layout input and the
 *    positioned output, both as JSON copies.
 * 2. Writes one case per fixture and per generated mindmap to crates/ferrigura-cose/tests/corpus/,
 *    with manifest.json. The directory is cleared of JSON first, so a re-run leaves no stale case.
 *
 * Run from the repository root:
 *
 *   pnpm exec tsx crates/ferrigura-cose/tools/capture-corpus.ts
 *
 * It starts `pnpm dev` on the port the e2e setup derives for this checkout, or reuses a dev server
 * that is already answering there, and stops it when done.
 */
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { basename, dirname, join, posix, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium, type Browser } from 'playwright';
import {
  GENERATED_COUNT,
  GENERATOR_SEED,
  generateMindmaps,
  type GeneratedMindmap,
} from './generate-trees.js';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const CORPUS_DIR = join(REPO_ROOT, 'crates/ferrigura-cose/tests/corpus');
const MINDMAP_FIXTURE_DIR = join(REPO_ROOT, 'e2e/diagrams/mindmap');

/** Version of the case and manifest format. Bump it when either changes shape. */
const CORPUS_VERSION = 1;
/** Indentation of the written JSON. */
const JSON_INDENT = 1;

const RENDER_TIMEOUT_MS = 30_000;
const SERVER_BOOT_TIMEOUT_MS = 300_000;
const VIEWPORT = { width: 1440, height: 1024 };

// ─────────────────────────────────────────────────────────────────────────────
// Corpus format (read by the Rust tests in crates/ferrigura-cose/tests)
// ─────────────────────────────────────────────────────────────────────────────

interface CorpusNode {
  id: string;
  width: number;
  height: number;
}

interface CorpusEdge {
  id: string;
  source: string;
  target: string;
}

interface CorpusPosition {
  id: string;
  x: number;
  y: number;
}

interface CorpusEdgePoints {
  id: string;
  startX: number;
  startY: number;
  midX: number;
  midY: number;
  endX: number;
  endY: number;
}

type CaseGroup = 'fixture' | 'generated';

interface CorpusCase {
  name: string;
  group: CaseGroup;
  /** Repository path of the fixture, or the generator seed and index. */
  source: string;
  text: string;
  input: { nodes: CorpusNode[]; edges: CorpusEdge[] };
  output: { positions: CorpusPosition[]; edgePoints: CorpusEdgePoints[] };
}

/** The hook's record of one cose-bilkent layout call, as the browser sent it. */
interface CapturedCall {
  input: { nodes: CorpusNode[]; edges: CorpusEdge[] };
  output: { nodes: CorpusPosition[]; edges: CorpusEdgePoints[] };
}

function byName(a: { name: string }, b: { name: string }): number {
  return a.name < b.name ? -1 : a.name > b.name ? 1 : 0;
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^\da-z]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

/** Case file content. `text` comes before `input`, and nothing else is added. */
function caseFileContent(record: CorpusCase): string {
  const body = {
    name: record.name,
    group: record.group,
    source: record.source,
    text: record.text,
    input: record.input,
    output: record.output,
  };
  return `${JSON.stringify(body, null, JSON_INDENT)}\n`;
}

/**
 * Clears the corpus directory of JSON and writes the records and the manifest. The directory is
 * generated output only, so nothing else in it is touched.
 */
function writeCorpus(records: CorpusCase[]): number {
  const sorted = [...records].sort(byName);
  const names = new Set(sorted.map((record) => record.name));
  if (names.size !== sorted.length) {
    throw new Error('Duplicate case names; two captures share a name');
  }
  mkdirSync(CORPUS_DIR, { recursive: true });
  for (const file of readdirSync(CORPUS_DIR)) {
    if (file.endsWith('.json')) {
      rmSync(join(CORPUS_DIR, file));
    }
  }
  for (const record of sorted) {
    writeFileSync(join(CORPUS_DIR, `${record.name}.json`), caseFileContent(record));
  }
  const manifest = {
    version: CORPUS_VERSION,
    cases: sorted.map((record) => ({
      name: record.name,
      group: record.group,
      source: record.source,
      nodes: record.input.nodes.length,
      edges: record.input.edges.length,
    })),
  };
  writeFileSync(
    join(CORPUS_DIR, 'manifest.json'),
    `${JSON.stringify(manifest, null, JSON_INDENT)}\n`
  );
  return readdirSync(CORPUS_DIR)
    .filter((file) => file.endsWith('.json'))
    .reduce((sum, file) => sum + statSync(join(CORPUS_DIR, file)).size, 0);
}

/**
 * Checks one captured call before it is written: ids match between input and output, the counts
 * agree, and every number is a finite double. JSON would turn NaN into null, which the Rust reader
 * rejects, so a non-finite value is a failure here rather than a silent change.
 */
function checkCall(call: CapturedCall): string | undefined {
  const numbers: unknown[] = [
    ...call.input.nodes.flatMap((node) => [node.width, node.height]),
    ...call.output.nodes.flatMap((node) => [node.x, node.y]),
    ...call.output.edges.flatMap((edge) => [
      edge.startX,
      edge.startY,
      edge.midX,
      edge.midY,
      edge.endX,
      edge.endY,
    ]),
  ];
  if (numbers.some((value) => typeof value !== 'number' || !Number.isFinite(value))) {
    return 'a width, height or position is missing or not finite';
  }
  const inputNodeIds = call.input.nodes.map((node) => node.id);
  const outputNodeIds = call.output.nodes.map((node) => node.id);
  if (new Set(inputNodeIds).size !== inputNodeIds.length) {
    return 'duplicate node ids in the input';
  }
  if (
    inputNodeIds.length !== outputNodeIds.length ||
    outputNodeIds.some((id) => !inputNodeIds.includes(id))
  ) {
    return `${outputNodeIds.length} positions for ${inputNodeIds.length} input nodes`;
  }
  const inputEdgeIds = call.input.edges.map((edge) => edge.id);
  const outputEdgeIds = call.output.edges.map((edge) => edge.id);
  if (
    inputEdgeIds.length !== outputEdgeIds.length ||
    outputEdgeIds.some((id) => !inputEdgeIds.includes(id))
  ) {
    return `${outputEdgeIds.length} edge results for ${inputEdgeIds.length} input edges`;
  }
  return undefined;
}

function caseFromCall(
  name: string,
  group: CaseGroup,
  source: string,
  text: string,
  call: CapturedCall
): CorpusCase {
  return {
    name,
    group,
    source,
    text,
    input: {
      nodes: call.input.nodes.map((node) => ({
        id: node.id,
        width: node.width,
        height: node.height,
      })),
      edges: call.input.edges.map((edge) => ({
        id: edge.id,
        source: edge.source,
        target: edge.target,
      })),
    },
    output: {
      positions: call.output.nodes.map((node) => ({ id: node.id, x: node.x, y: node.y })),
      edgePoints: call.output.edges.map((edge) => ({
        id: edge.id,
        startX: edge.startX,
        startY: edge.startY,
        midX: edge.midX,
        midY: edge.midY,
        endX: edge.endX,
        endY: edge.endY,
      })),
    },
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixture selection
// ─────────────────────────────────────────────────────────────────────────────

interface Fixture {
  /** Repository path, as the manifest's source. */
  relPath: string;
  text: string;
}

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
  return findMmdFiles(MINDMAP_FIXTURE_DIR).map((file) => ({
    relPath: posix.join(...relative(REPO_ROOT, file).split(sep)),
    text: readFileSync(file, 'utf8'),
  }));
}

/** Case name of a fixture: its path under e2e/diagrams/mindmap/, each segment turned into a slug. */
function fixtureName(relPath: string): string {
  return relPath
    .replace(/^e2e\/diagrams\/mindmap\//, '')
    .replace(/\.mmd$/, '')
    .split('/')
    .map((segment) => slugify(segment))
    .join('__');
}

// ─────────────────────────────────────────────────────────────────────────────
// Dev server (mirrors the port logic of .esbuild/server.ts, as capture-corpus.ts for ELK does)
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

const sleep = (ms: number) => new Promise((done) => setTimeout(done, ms));

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

  // The server prints "Rebuild N Time (total)" once its first build has finished, success or not.
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
  return Buffer.from(JSON.stringify({ code, mermaid: {} }), 'utf8').toString('base64');
}

type RenderOutcome =
  | { status: 'captured'; calls: CapturedCall[] }
  | { status: 'skipped'; category: string; detail: string };

async function renderWithCoseCapture(
  browser: Browser,
  baseUrl: string,
  text: string
): Promise<RenderOutcome> {
  // A new page per case: the capture array starts empty and nothing leaks across cases.
  const page = await browser.newPage({ viewport: VIEWPORT });
  const pageErrors: string[] = [];
  page.on('pageerror', (error) => pageErrors.push(error.message));
  try {
    await page.addInitScript(() => {
      // Installed before Mermaid loads. layout.ts reads the hook at each layout call.
      const target = globalThis as unknown as {
        __mermaidCoseCapture?: (input: unknown, output: unknown) => void;
        mermaidCoseCaptures?: { input: unknown; output: unknown }[];
      };
      target.mermaidCoseCaptures = [];
      target.__mermaidCoseCapture = (input, output) => {
        target.mermaidCoseCaptures?.push({ input, output });
      };
    });
    // A large mindmap makes the ?graph= URL longer than Node's 16 KB request-header limit, and the
    // dev server never answers it. The page reads the graph from location.href only, so the request
    // is answered with e2e.html fetched without the query; the page and the bundle are unchanged.
    await page.route(
      (url) => url.pathname === '/e2e.html' && url.search.startsWith('?graph='),
      async (route) => {
        const response = await route.fetch({ url: `${baseUrl}/e2e.html` });
        await route.fulfill({ response });
      }
    );
    try {
      await page.goto(`${baseUrl}/e2e.html?graph=${encodeGraph(text)}`, { waitUntil: 'load' });
    } catch (error) {
      return {
        status: 'skipped',
        category: 'navigation failed',
        detail: String(error).slice(0, 300),
      };
    }
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
        (globalThis as unknown as { mermaidCoseCaptures?: CapturedCall[] }).mermaidCoseCaptures ??
        []
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
    if (roledescription !== 'mindmap') {
      return {
        status: 'skipped',
        category: 'not rendered as a mindmap',
        detail: `aria-roledescription=${roledescription}`,
      };
    }
    if (calls.length === 0) {
      return {
        status: 'skipped',
        category: 'no cose layout call',
        detail: 'the layout did not go through cose-bilkent',
      };
    }
    return { status: 'captured', calls };
  } finally {
    await page.close();
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

async function main(): Promise<void> {
  const fixtures = selectFixtures();
  const generated: GeneratedMindmap[] = generateMindmaps(GENERATOR_SEED, GENERATED_COUNT);
  console.log(
    `Selected ${fixtures.length} mindmap fixtures and ${generated.length} generated mindmaps (seed ${GENERATOR_SEED})`
  );

  const records: CorpusCase[] = [];
  const skipped: { fixture: string; category: string; detail: string }[] = [];
  const failures: string[] = [];

  const server = await ensureDevServer();
  const browser = await chromium.launch({ headless: true });
  try {
    for (const [i, fixture] of fixtures.entries()) {
      const name = fixtureName(fixture.relPath);
      const outcome = await renderWithCoseCapture(browser, server.baseUrl, fixture.text);
      if (outcome.status === 'skipped') {
        skipped.push({
          fixture: fixture.relPath,
          category: outcome.category,
          detail: outcome.detail,
        });
        console.log(
          `[fixture ${i + 1}/${fixtures.length}] skip ${fixture.relPath}: ${outcome.category}`
        );
        continue;
      }
      for (const [k, call] of outcome.calls.entries()) {
        const caseName = outcome.calls.length === 1 ? name : `${name}__${k}`;
        const problem = checkCall(call);
        if (problem !== undefined) {
          failures.push(`${caseName} (${fixture.relPath}): ${problem}`);
          continue;
        }
        records.push(caseFromCall(caseName, 'fixture', fixture.relPath, fixture.text, call));
      }
      console.log(
        `[fixture ${i + 1}/${fixtures.length}] ok   ${fixture.relPath} (${outcome.calls.length} layout call${outcome.calls.length === 1 ? '' : 's'})`
      );
    }

    for (const mindmap of generated) {
      const name = `generated-${String(mindmap.index).padStart(3, '0')}`;
      const source = `seed ${GENERATOR_SEED} index ${mindmap.index}`;
      if (mindmap.index % 25 === 0) {
        console.log(`[generated ${mindmap.index + 1}/${generated.length}] rendering ${name}`);
      }
      const outcome = await renderWithCoseCapture(browser, server.baseUrl, mindmap.text);
      if (outcome.status === 'skipped') {
        failures.push(`${name}: ${outcome.category} (${outcome.detail})`);
        continue;
      }
      if (outcome.calls.length !== 1) {
        failures.push(`${name}: ${outcome.calls.length} layout calls, expected 1`);
        continue;
      }
      const call = outcome.calls[0];
      const nodeCount = mindmap.parents.length;
      if (call.input.nodes.length !== nodeCount || call.input.edges.length !== nodeCount - 1) {
        failures.push(
          `${name}: ${call.input.nodes.length} nodes and ${call.input.edges.length} edges, expected ${nodeCount} and ${nodeCount - 1}`
        );
        continue;
      }
      const problem = checkCall(call);
      if (problem !== undefined) {
        failures.push(`${name}: ${problem}`);
        continue;
      }
      records.push(caseFromCall(name, 'generated', source, mindmap.text, call));
    }
  } finally {
    await browser.close();
    server.stop();
  }

  if (failures.length > 0) {
    throw new Error(
      `Capture failed for ${failures.length} case(s); nothing was written:\n  ${failures.join('\n  ')}`
    );
  }

  const totalBytes = writeCorpus(records);
  const fixtureCount = records.filter((record) => record.group === 'fixture').length;
  const generatedCount = records.filter((record) => record.group === 'generated').length;

  console.log('');
  console.log(
    `Cases written: ${records.length} (${fixtureCount} fixture, ${generatedCount} generated) + manifest.json to ${relative(REPO_ROOT, CORPUS_DIR)}`
  );
  console.log(`Total size: ${(totalBytes / 1_000_000).toFixed(2)} MB`);
  console.log(`Fixtures skipped: ${skipped.length}`);
  const byCategory = new Map<string, { fixture: string; detail: string }[]>();
  for (const entry of skipped) {
    byCategory.set(entry.category, [
      ...(byCategory.get(entry.category) ?? []),
      { fixture: entry.fixture, detail: entry.detail },
    ]);
  }
  for (const [category, entries] of [...byCategory].sort(([a], [b]) => (a < b ? -1 : 1))) {
    console.log(`  ${category}: ${entries.length}`);
    for (const entry of entries) {
      console.log(`    ${entry.fixture} (${entry.detail})`);
    }
  }
}

main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
