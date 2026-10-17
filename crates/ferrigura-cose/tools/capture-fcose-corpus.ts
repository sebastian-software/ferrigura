/* eslint-disable no-console -- CLI tool: progress and results go to the console */
/**
 * Differential corpus of the cytoscape-fcose layout for the ferrigura-cose port.
 *
 * 1. Renders the architecture diagrams in Chromium: the fixtures under e2e/diagrams/architecture/,
 *    each diagram text in the architecture specs, and the generated diagrams
 *    (generate-architectures.ts). The page gets a `__mermaidFcoseCapture` hook before Mermaid loads.
 *    architectureRenderer.ts calls it four times per render: the input of run 1, the positions after
 *    run 1, the input of run 2, and the positions after run 2.
 * 2. Pairs the four records into one case per render with two runs. Each case is checked before it
 *    is written: finite numbers, unique ids, output ids in input order, and both runs describing the
 *    same graph. A failed check stops the write. A render that does not produce an architecture
 *    diagram with exactly two runs is a skip, with a category.
 * 3. Writes crates/ferrigura-cose/tests/fcose-corpus/<name>.json and manifest.json. The directory is
 *    cleared of JSON first, so a re-run leaves no stale case.
 *
 * Run from the repository root, after `pnpm build:esbuild` if architectureRenderer.ts changed:
 *
 *   pnpm exec tsx crates/ferrigura-cose/tools/capture-fcose-corpus.ts
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
import ts from 'typescript';
import {
  GENERATED_COUNT,
  GENERATOR_SEED,
  generateArchitectures,
} from './generate-architectures.js';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const CORPUS_DIR = join(REPO_ROOT, 'crates/ferrigura-cose/tests/fcose-corpus');
const FIXTURE_DIR = join(REPO_ROOT, 'e2e/diagrams/architecture');
const SPEC_DIR = join(REPO_ROOT, 'packages/mermaid/src/diagrams/architecture');

/** Version of the case and manifest format. Bump it when either changes shape. */
const CORPUS_VERSION = 1;
/** Indentation of the written JSON. */
const JSON_INDENT = 1;
/** Records per render: input and output of run 1, then input and output of run 2. */
const RECORDS_PER_RENDER = 4;
/** Pages rendered at once. Each case still gets a page of its own. */
const PAGE_CONCURRENCY = 4;

const RENDER_TIMEOUT_MS = 30_000;
const SERVER_BOOT_TIMEOUT_MS = 300_000;
const VIEWPORT = { width: 1440, height: 1024 };

// ─────────────────────────────────────────────────────────────────────────────
// Corpus format (read by the Rust tests in crates/ferrigura-cose/tests)
// ─────────────────────────────────────────────────────────────────────────────

interface CorpusNode {
  id: string;
  parent: string | null;
  width: number;
  height: number;
  padding: number;
  x: number;
  y: number;
}

interface CorpusEdge {
  id: string;
  source: string;
  target: string;
  idealLength: number;
  elasticity: number;
}

interface CorpusPosition {
  id: string;
  x: number;
  y: number;
}

interface CorpusOptions {
  quality: string;
  randomize: boolean;
  nodeSeparation: number;
  numIter: number;
  nodeDimensionsIncludeLabels: boolean;
}

interface CorpusBounds {
  x1: number;
  y1: number;
  w: number;
  h: number;
}

interface CorpusRun {
  run: 1 | 2;
  seed: number;
  options: CorpusOptions;
  nodes: CorpusNode[];
  edges: CorpusEdge[];
  /** Omitted when the layout has no such constraint. */
  alignmentConstraint?: unknown;
  /** Omitted when the layout has no such constraint. */
  relativePlacementConstraint?: unknown;
  /** The bounding box of the layout's elements at the start of the run. */
  bounds: CorpusBounds;
  /** The positions after the run, in node order. */
  output: CorpusPosition[];
}

type CaseGroup = 'fixture' | 'spec' | 'generated';

interface CorpusCase {
  name: string;
  group: CaseGroup;
  /** Repository path and line, or the generator seed and index. */
  source: string;
  text: string;
  runs: CorpusRun[];
}

/** One record of the hook, as the page sent it (a plain JSON copy). */
interface FcoseRecord {
  stage: 'input' | 'output';
  run: number;
  [key: string]: unknown;
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

/** Case file content. Keys come in the order of the case shape, and nothing else is added. */
function caseFileContent(record: CorpusCase): string {
  const body = {
    name: record.name,
    group: record.group,
    source: record.source,
    text: record.text,
    runs: record.runs,
  };
  return `${JSON.stringify(body, null, JSON_INDENT)}\n`;
}

/**
 * Clears the corpus directory of JSON and writes the cases and the manifest. The directory is
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
      seed: record.runs[0].seed,
      nodes: record.runs[0].nodes.length,
      edges: record.runs[0].edges.length,
      runs: record.runs.length,
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

// ─────────────────────────────────────────────────────────────────────────────
// Diagram selection
// ─────────────────────────────────────────────────────────────────────────────

/** One diagram to render. `skip` is set when the text cannot be captured as it stands. */
interface Job {
  name: string;
  group: CaseGroup;
  source: string;
  text: string;
  /** The `mermaid` key of the e2e payload: the architecture config, or nothing. */
  mermaid: Record<string, unknown>;
  skip?: { category: string; detail: string };
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

/** The architecture fixtures: one job per .mmd file. The case name is the file's slug. */
function fixtureJobs(): Job[] {
  return findMmdFiles(FIXTURE_DIR).map((file) => {
    const stem = basename(file, '.mmd');
    return {
      name: slugify(stem),
      group: 'fixture',
      source: posix.join(...relative(REPO_ROOT, file).split(sep)),
      text: readFileSync(file, 'utf8'),
      mermaid: {},
    };
  });
}

/**
 * The diagram texts of the architecture spec files: every string or template literal that contains
 * `architecture-beta`. A template with substitutions cannot be captured as written, so it is a
 * skip rather than a guess at its value. The name is `spec-<file>-<NN>`, with NN counting the texts
 * of that file in source order.
 */
function specJobs(): Job[] {
  const jobs: Job[] = [];
  const files = readdirSync(SPEC_DIR)
    .filter((file) => file.endsWith('.spec.ts'))
    .sort();
  for (const file of files) {
    const sourceFile = ts.createSourceFile(
      file,
      readFileSync(join(SPEC_DIR, file), 'utf8'),
      ts.ScriptTarget.Latest,
      true
    );
    const found: { text: string; line: number; substituted: boolean }[] = [];
    const lineOf = (node: ts.Node) =>
      sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1;
    const visit = (node: ts.Node): void => {
      if (ts.isNoSubstitutionTemplateLiteral(node) || ts.isStringLiteral(node)) {
        if (node.text.includes('architecture-beta')) {
          found.push({ text: node.text, line: lineOf(node), substituted: false });
        }
      } else if (ts.isTemplateExpression(node) && node.head.text.includes('architecture-beta')) {
        found.push({ text: node.getText(sourceFile), line: lineOf(node), substituted: true });
      }
      ts.forEachChild(node, visit);
    };
    visit(sourceFile);
    const stem = slugify(file.replace(/\.spec\.ts$/, ''));
    found.forEach((entry, i) => {
      jobs.push({
        name: `spec-${stem}-${String(i + 1).padStart(2, '0')}`,
        group: 'spec',
        source: `packages/mermaid/src/diagrams/architecture/${file}:${entry.line}`,
        text: entry.text,
        mermaid: {},
        skip: entry.substituted
          ? {
              category: 'template with substitutions',
              detail: 'the text is built at run time, so it is not captured',
            }
          : undefined,
      });
    });
  }
  return jobs;
}

/** The generated diagrams, each with its own layout config in the payload. */
function generatedJobs(): Job[] {
  return generateArchitectures(GENERATOR_SEED, GENERATED_COUNT).map((diagram) => ({
    name: `generated-${String(diagram.index).padStart(3, '0')}`,
    group: 'generated',
    source: `seed ${GENERATOR_SEED} index ${diagram.index}`,
    text: diagram.text,
    mermaid: { architecture: diagram.config },
  }));
}

// ─────────────────────────────────────────────────────────────────────────────
// Dev server (mirrors capture-corpus.ts for the mindmap layout)
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
function encodeGraph(code: string, mermaid: Record<string, unknown>): string {
  return Buffer.from(JSON.stringify({ code, mermaid }), 'utf8').toString('base64');
}

type RenderOutcome =
  | { status: 'captured'; records: FcoseRecord[] }
  | { status: 'skipped'; category: string; detail: string };

async function renderWithFcoseCapture(
  browser: Browser,
  baseUrl: string,
  job: Job
): Promise<RenderOutcome> {
  // A new page per case: the record array starts empty and nothing leaks across cases.
  const page = await browser.newPage({ viewport: VIEWPORT });
  const pageErrors: string[] = [];
  page.on('pageerror', (error) => pageErrors.push(error.message));
  try {
    await page.addInitScript(() => {
      // Installed before Mermaid loads. architectureRenderer.ts reads the hook at each layout.
      const target = globalThis as unknown as {
        __mermaidFcoseCapture?: (record: unknown) => void;
        mermaidFcoseRecords?: unknown[];
        mermaidFcoseRejections?: string[];
      };
      target.mermaidFcoseRecords = [];
      target.mermaidFcoseRejections = [];
      target.__mermaidFcoseCapture = (record) => {
        target.mermaidFcoseRecords?.push(record);
      };
      // Mermaid's parse and validation errors are thrown as plain objects, which reach the page as an
      // unhandled rejection with the message "Object". The message is kept for the skip report.
      globalThis.addEventListener('unhandledrejection', (event) => {
        const reason: unknown = event.reason;
        const message =
          typeof reason === 'object' && reason !== null && 'message' in reason
            ? String(reason.message)
            : String(reason);
        target.mermaidFcoseRejections?.push(message);
      });
    });
    // A long ?graph= URL can be too long for the dev server's request-header limit, and the server
    // then never answers it. The page reads the graph from location.href only, so the request is
    // answered with e2e.html fetched without the query; the page and the bundle are unchanged.
    await page.route(
      (url) => url.pathname === '/e2e.html' && url.search.startsWith('?graph='),
      async (route) => {
        const response = await route.fetch({ url: `${baseUrl}/e2e.html` });
        await route.fulfill({ response });
      }
    );
    try {
      await page.goto(`${baseUrl}/e2e.html?graph=${encodeGraph(job.text, job.mermaid)}`, {
        waitUntil: 'load',
      });
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
    const records = await page.evaluate(
      () =>
        (globalThis as unknown as { mermaidFcoseRecords?: FcoseRecord[] }).mermaidFcoseRecords ?? []
    );
    const rejections = await page.evaluate(
      () =>
        (globalThis as unknown as { mermaidFcoseRejections?: string[] }).mermaidFcoseRejections ??
        []
    );

    if (pageErrors.length > 0) {
      const detail = rejections[0] ?? pageErrors[0];
      return { status: 'skipped', category: 'page error', detail: detail.slice(0, 300) };
    }
    if (roledescription === null) {
      return { status: 'skipped', category: 'no diagram svg', detail: 'page has no diagram' };
    }
    if (roledescription === 'error') {
      const text = await page
        .locator('svg[aria-roledescription=error]')
        .first()
        .textContent()
        .catch(() => '');
      return {
        status: 'skipped',
        category: 'rendered as error diagram',
        detail: (text ?? '').replace(/\s+/g, ' ').trim().slice(0, 300),
      };
    }
    if (roledescription !== 'architecture') {
      return {
        status: 'skipped',
        category: 'not rendered as architecture',
        detail: `aria-roledescription=${roledescription}`,
      };
    }
    if (records.length === 0) {
      return {
        status: 'skipped',
        category: 'no fcose layout run',
        detail: 'the architecture layout did not reach the fcose hook',
      };
    }
    if (records.length !== RECORDS_PER_RENDER) {
      return {
        status: 'skipped',
        category: 'unexpected fcose records',
        detail: `${records.length} records, expected ${RECORDS_PER_RENDER}`,
      };
    }
    return { status: 'captured', records };
  } finally {
    await page.close();
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pairing, checks and probes
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Pairs the four records of a render into two runs. Each run is its input record followed by its
 * output record. Returns the reason when the records are not in that order.
 */
function pairRuns(records: FcoseRecord[]): { runs: CorpusRun[] } | { problem: string } {
  const runs: CorpusRun[] = [];
  for (const run of [1, 2] as const) {
    const input = records[(run - 1) * 2];
    const output = records[(run - 1) * 2 + 1];
    if (input.stage !== 'input' || input.run !== run) {
      return { problem: `record ${(run - 1) * 2} is not the input of run ${run}` };
    }
    if (output.stage !== 'output' || output.run !== run) {
      return { problem: `record ${(run - 1) * 2 + 1} is not the output of run ${run}` };
    }
    // The keys come in the order of the case shape (CorpusRun), so a diff reads like the format.
    const built: CorpusRun = {
      run,
      seed: input.seed as number,
      options: input.options as CorpusOptions,
      nodes: input.nodes as CorpusNode[],
      edges: input.edges as CorpusEdge[],
      ...(input.alignmentConstraint === undefined
        ? {}
        : { alignmentConstraint: input.alignmentConstraint }),
      ...(input.relativePlacementConstraint === undefined
        ? {}
        : { relativePlacementConstraint: input.relativePlacementConstraint }),
      bounds: input.bounds as CorpusBounds,
      output: output.positions as CorpusPosition[],
    };
    runs.push(built);
  }
  return { runs };
}

/** The first path of a value that is not a finite number, or null where a number is needed. */
function findBadValue(value: unknown, path: string): string | undefined {
  if (value === null) {
    return path;
  }
  if (typeof value === 'number') {
    return Number.isFinite(value) ? undefined : path;
  }
  if (Array.isArray(value)) {
    for (const [i, item] of value.entries()) {
      const bad = findBadValue(item, `${path}[${i}]`);
      if (bad !== undefined) {
        return bad;
      }
    }
    return undefined;
  }
  if (typeof value === 'object' && value !== null) {
    for (const [key, item] of Object.entries(value)) {
      // A node without a parent is the only null the hook writes.
      if (key === 'parent' && item === null) {
        continue;
      }
      const bad = findBadValue(item, `${path}.${key}`);
      if (bad !== undefined) {
        return bad;
      }
    }
  }
  return undefined;
}

const sameJson = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/** The first failed check of a case, or undefined when every check passes. */
function checkRuns(runs: CorpusRun[]): string | undefined {
  for (const run of runs) {
    const bad = findBadValue(run, `run ${run.run}`);
    if (bad !== undefined) {
      return `a value is not a finite number at ${bad}`;
    }
  }
  const [first, second] = runs;
  const nodeIds = first.nodes.map((node) => node.id);
  if (new Set(nodeIds).size !== nodeIds.length) {
    return 'duplicate node ids';
  }
  const edgeIds = first.edges.map((edge) => edge.id);
  if (new Set(edgeIds).size !== edgeIds.length) {
    return 'duplicate edge ids';
  }
  const known = new Set(nodeIds);
  for (const edge of first.edges) {
    if (!known.has(edge.source) || !known.has(edge.target)) {
      return `edge ${edge.id} has an endpoint that is not a node`;
    }
  }
  for (const run of runs) {
    const outputIds = run.output.map((position) => position.id);
    if (!sameJson(outputIds, nodeIds)) {
      return `run ${run.run}: the output ids are not the node ids in the same order`;
    }
  }
  if (!sameJson(second.seed, first.seed) || !sameJson(second.options, first.options)) {
    return 'the two runs have different seeds or options';
  }
  const shape = (run: CorpusRun) => run.nodes.map((node) => ({ id: node.id, parent: node.parent }));
  if (!sameJson(shape(second), shape(first))) {
    return 'the two runs have different nodes, or the nodes are in a different order';
  }
  const edgeShape = (run: CorpusRun) =>
    run.edges.map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      idealLength: edge.idealLength,
      elasticity: edge.elasticity,
    }));
  if (!sameJson(edgeShape(second), edgeShape(first))) {
    return 'the two runs have different edges, or the edges are in a different order';
  }
  if (
    !sameJson(second.alignmentConstraint, first.alignmentConstraint) ||
    !sameJson(second.relativePlacementConstraint, first.relativePlacementConstraint)
  ) {
    return 'the two runs have different constraints';
  }
  // A node without children has the same size before and after a run. A compound node's size
  // comes from its children, so only the simple nodes are required to match.
  const parents = new Set(first.nodes.map((node) => node.parent).filter((p) => p !== null));
  for (const [i, node] of first.nodes.entries()) {
    if (parents.has(node.id)) {
      continue;
    }
    const other = second.nodes[i];
    if (
      other.width !== node.width ||
      other.height !== node.height ||
      other.padding !== node.padding
    ) {
      return `simple node ${node.id} changes size between the runs`;
    }
  }
  return undefined;
}

/** The id declarations of the diagram text, by kind, in declaration order. */
function declaredIds(text: string): { groups: string[]; services: string[]; junctions: string[] } {
  const ids = { groups: [] as string[], services: [] as string[], junctions: [] as string[] };
  for (const line of text.split('\n')) {
    const match = /^\s*(group|service|junction)\s+([^\s([]+)/.exec(line);
    if (match?.[1] === 'group') {
      ids.groups.push(match[2]);
    } else if (match?.[1] === 'service') {
      ids.services.push(match[2]);
    } else if (match?.[1] === 'junction') {
      ids.junctions.push(match[2]);
    }
  }
  return ids;
}

interface Probes {
  /** Cases whose node order differs from the add order: groups, then services, then junctions. */
  addOrderMismatch: string[];
  /** Cases with an edge whose endpoint is a group id. */
  groupEndpoint: string[];
  /** Runs whose bounding box has zero width. */
  zeroWidthBounds: string[];
  /** Cases with randomize true and fewer than three nodes. */
  randomizeUnderThreeNodes: string[];
  /** Cases where run 2 leaves every position as run 1 left it. */
  run2PositionsUnchanged: string[];
  /** Cases where a compound node changes size between the runs. */
  compoundSizeChanged: string[];
  /** Cases where the layout options have randomize true. */
  randomizeTrue: string[];
  /** Cases whose alignment constraint has no row and no column. */
  noAlignmentConstraint: string[];
  /** Cases whose relative placement constraint is empty. */
  noRelativeConstraint: string[];
}

function probeCase(name: string, text: string, runs: CorpusRun[], probes: Probes): void {
  const [first, second] = runs;
  const ids = declaredIds(text);
  const expectedOrder = [...ids.groups, ...ids.services, ...ids.junctions];
  if (
    !sameJson(
      first.nodes.map((node) => node.id),
      expectedOrder
    )
  ) {
    probes.addOrderMismatch.push(name);
  }
  const groupIds = new Set(ids.groups);
  if (first.edges.some((edge) => groupIds.has(edge.source) || groupIds.has(edge.target))) {
    probes.groupEndpoint.push(name);
  }
  for (const run of runs) {
    if (run.bounds.w === 0) {
      probes.zeroWidthBounds.push(`${name} run ${run.run}`);
    }
  }
  if (first.options.randomize) {
    probes.randomizeTrue.push(name);
    if (first.nodes.length < 3) {
      probes.randomizeUnderThreeNodes.push(name);
    }
  }
  // getAlignments returns { horizontal: [], vertical: [] } and getRelativeConstraints returns []
  // when there is nothing to place, so an empty constraint is present in the record.
  const alignment = first.alignmentConstraint as { horizontal?: unknown[]; vertical?: unknown[] };
  if ((alignment.horizontal ?? []).length === 0 && (alignment.vertical ?? []).length === 0) {
    probes.noAlignmentConstraint.push(name);
  }
  if ((first.relativePlacementConstraint as unknown[]).length === 0) {
    probes.noRelativeConstraint.push(name);
  }
  if (sameJson(first.output, second.output)) {
    probes.run2PositionsUnchanged.push(name);
  }
  const parents = new Set(first.nodes.map((node) => node.parent).filter((p) => p !== null));
  const changed = first.nodes.some(
    (node, i) =>
      parents.has(node.id) &&
      (second.nodes[i].width !== node.width || second.nodes[i].height !== node.height)
  );
  if (changed) {
    probes.compoundSizeChanged.push(name);
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

/** Runs `fn` over `items` with at most `size` at once. Results keep the order of `items`. */
async function mapPool<T, R>(
  items: readonly T[],
  size: number,
  fn: (item: T) => Promise<R>
): Promise<R[]> {
  const results: R[] = [];
  let next = 0;
  const worker = async (): Promise<void> => {
    while (next < items.length) {
      const i = next++;
      results[i] = await fn(items[i]);
    }
  };
  await Promise.all(Array.from({ length: Math.min(size, items.length) }, worker));
  return results;
}

interface Skip {
  name: string;
  group: CaseGroup;
  source: string;
  category: string;
  detail: string;
}

async function main(): Promise<void> {
  const fixtures = fixtureJobs();
  const specs = specJobs();
  const generated = generatedJobs();
  const jobs = [...fixtures, ...specs, ...generated];
  console.log(
    `Selected ${fixtures.length} fixtures, ${specs.length} spec texts and ${generated.length} generated diagrams (seed ${GENERATOR_SEED})`
  );

  const cases: CorpusCase[] = [];
  const skips: Skip[] = [];
  const failures: string[] = [];
  const probes: Probes = {
    addOrderMismatch: [],
    groupEndpoint: [],
    zeroWidthBounds: [],
    randomizeUnderThreeNodes: [],
    run2PositionsUnchanged: [],
    compoundSizeChanged: [],
    randomizeTrue: [],
    noAlignmentConstraint: [],
    noRelativeConstraint: [],
  };

  const server = await ensureDevServer();
  const browser = await chromium.launch({ headless: true });
  let done = 0;
  try {
    await mapPool(jobs, PAGE_CONCURRENCY, async (job) => {
      const skipOf = (category: string, detail: string) =>
        skips.push({ name: job.name, group: job.group, source: job.source, category, detail });
      if (job.skip !== undefined) {
        skipOf(job.skip.category, job.skip.detail);
      } else {
        const outcome = await renderWithFcoseCapture(browser, server.baseUrl, job);
        if (outcome.status === 'skipped') {
          skipOf(outcome.category, outcome.detail);
        } else {
          const paired = pairRuns(outcome.records);
          if ('problem' in paired) {
            skipOf('unexpected fcose records', paired.problem);
          } else if (paired.runs[0].nodes.length === 0) {
            skipOf('no nodes', 'the layout had no nodes to place');
          } else if (
            paired.runs.some(
              (run) =>
                ![run.bounds.x1, run.bounds.y1, run.bounds.w, run.bounds.h].every(Number.isFinite)
            )
          ) {
            skipOf('empty bounding box', 'the layout elements had no bounding box');
          } else {
            const problem = checkRuns(paired.runs);
            if (problem !== undefined) {
              failures.push(`${job.name} (${job.source}): ${problem}`);
              console.log(`check failed: ${job.name} (${job.source}): ${problem}`);
            } else {
              cases.push({
                name: job.name,
                group: job.group,
                source: job.source,
                text: job.text,
                runs: paired.runs,
              });
              probeCase(job.name, job.text, paired.runs, probes);
            }
          }
        }
      }
      done++;
      if (done % 50 === 0 || done === jobs.length) {
        console.log(`[${done}/${jobs.length}] rendered`);
      }
    });
  } finally {
    await browser.close();
    server.stop();
  }

  if (failures.length > 0) {
    throw new Error(
      `Capture failed for ${failures.length} case(s); nothing was written:\n  ${failures.join('\n  ')}`
    );
  }

  const totalBytes = writeCorpus(cases);
  const counts = (group: CaseGroup) => ({
    captured: cases.filter((record) => record.group === group).length,
    total: jobs.filter((job) => job.group === group).length,
  });
  console.log('');
  console.log(
    `Cases written: ${cases.length} (fixture ${counts('fixture').captured}/${counts('fixture').total}, spec ${counts('spec').captured}/${counts('spec').total}, generated ${counts('generated').captured}/${counts('generated').total}) + manifest.json to ${relative(REPO_ROOT, CORPUS_DIR)}`
  );
  console.log(`Total size: ${(totalBytes / 1_000_000).toFixed(2)} MB`);

  console.log(`Skipped: ${skips.length}`);
  const byCategory = new Map<string, Skip[]>();
  for (const skip of skips) {
    byCategory.set(skip.category, [...(byCategory.get(skip.category) ?? []), skip]);
  }
  for (const [category, entries] of [...byCategory].sort(([a], [b]) => (a < b ? -1 : 1))) {
    console.log(`  ${category}: ${entries.length}`);
    console.log(`    e.g. ${entries[0].name} (${entries[0].detail})`);
  }

  console.log('Probes:');
  const show = (label: string, names: string[]) =>
    console.log(
      `  ${label}: ${names.length}${names.length > 0 ? ` (e.g. ${names.slice(0, 3).join(', ')})` : ''}`
    );
  show('node order differs from groups, services, junctions', probes.addOrderMismatch);
  show('edge with a group as endpoint', probes.groupEndpoint);
  show('runs with bounding box width 0', probes.zeroWidthBounds);
  show('randomize true', probes.randomizeTrue);
  show('randomize true with fewer than three nodes', probes.randomizeUnderThreeNodes);
  show('run 2 leaves all positions as run 1 left them', probes.run2PositionsUnchanged);
  show('compound node changes size between runs', probes.compoundSizeChanged);
  show('empty alignment constraint', probes.noAlignmentConstraint);
  show('empty relative placement constraint', probes.noRelativeConstraint);
}

main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
