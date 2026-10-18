/* eslint-disable no-console -- CLI tool: progress and results go to the console */
/**
 * Reference corpus of the swimlane layout (crates/ferrigura-swimlane/tests/corpus): the input, the
 * stages and the output of `runSwimlaneLayoutCore`, captured from the Mermaid 12.2.0 reference.
 *
 * 1. Chromium: renders the e2e swimlane fixtures (e2e/diagrams/swimlanes/) and the generated diagrams
 *    (generate-swimlanes.ts) with the stage hook installed before Mermaid loads. The page also has the
 *    reference harness's clock (1700000000000) and a seeded Math.random (seed 7), as in
 *    tools/port/geometry/common.ts, so two renders of one text give the same layout data.
 * 2. DDLT: runs capture-ddlt.spec.ts under vitest (the layout-test fixtures, through loadDdltFixture).
 * 3. Writes one case per core call and manifest.json, with the key set and the config keys. The
 *    directory is cleared of JSON first, so a re-run leaves no stale case.
 *
 * Run from the repository root:
 *
 *   pnpm exec tsx crates/ferrigura-swimlane/tools/capture-corpus.ts
 *
 * It starts `pnpm dev` on the port the e2e setup derives for this checkout, or reuses a dev server
 * already answering there, and stops it when done.
 *
 * Determinism check (a second capture of selected cases into a scratch directory; the DDLT part and
 * the corpus are not touched):
 *
 *   pnpm exec tsx crates/ferrigura-swimlane/tools/capture-corpus.ts --only pr-check__1-simple,generated-000 --out <dir>
 */
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import { basename, dirname, join, posix, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium, type Browser } from 'playwright';
import { generateSwimlanes, GENERATED_COUNT, GENERATOR_SEED } from './generate-swimlanes.js';
import {
  CONFIG_KEYS,
  CORPUS_DIR,
  PARTS_DIR,
  REPO_ROOT,
  SWIMLANE_SOURCE_DIR,
  casesFromCalls,
  identifierKeySet,
  sourceFiles,
  splitCalls,
  writeCorpus,
  type CaptureResult,
  type CorpusCase,
  type Exclusion,
  type StageRecord,
} from './corpus-format.js';

const TOOLS_DIR = dirname(fileURLToPath(import.meta.url));
const FIXTURE_DIR = join(REPO_ROOT, 'e2e/diagrams/swimlanes');
const FIXTURE_PREFIX = 'e2e/diagrams/swimlanes/';

const RENDER_TIMEOUT_MS = 30_000;
const SERVER_BOOT_TIMEOUT_MS = 300_000;
const VIEWPORT = { width: 1440, height: 1024 };

/** The front matter of the e2e fixtures, for the generated diagrams. */
const FIXTURE_FRONT_MATTER =
  '---\nconfig:\n  htmlLabels: false\n  flowchart:\n    useMaxWidth: false\n---\n';

/**
 * The page's stage hook, the clock and the random numbers. Plain script: it runs before Mermaid
 * loads. The clock and the seed are the reference harness's (renderReference: now 1700000000000,
 * seed 7); Math.random is the same generator as the harness's.
 */
const CAPTURE_INIT_SCRIPT = `
window.mermaidSwimlaneRecords = [];
window.__mermaidSwimlaneStage = function (stage, data) {
  window.mermaidSwimlaneRecords.push({ stage: stage, data: data });
};
(function () {
  const RealDate = Date;
  const fixedNow = 1700000000000;
  class FixedDate extends RealDate {
    constructor(...args) {
      if (args.length === 0) { super(fixedNow); } else { super(...args); }
    }
    static now() { return fixedNow; }
  }
  window.Date = FixedDate;
  let state = 7 >>> 0;
  Math.random = function () {
    state = (state + 0x6d2b79f5) | 0;
    let t = Math.imul(state ^ (state >>> 15), 1 | state);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
})();
`;

// ─────────────────────────────────────────────────────────────────────────────
// Names and selection
// ─────────────────────────────────────────────────────────────────────────────

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^\da-z]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

function byText(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

function findMmdFiles(dir: string): string[] {
  const files: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true }).sort((a, b) =>
    byText(a.name, b.name)
  )) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...findMmdFiles(full));
    } else if (entry.isFile() && entry.name.endsWith('.mmd')) {
      files.push(full);
    }
  }
  return files;
}

interface Fixture {
  /** Repository path, as the manifest's source. */
  relPath: string;
  /** Case name: the path under e2e/diagrams/swimlanes/, each segment turned into a slug, joined by `__`. */
  name: string;
  text: string;
}

function selectFixtures(): Fixture[] {
  return findMmdFiles(FIXTURE_DIR).map((file) => {
    const relPath = posix.join(...relative(REPO_ROOT, file).split(sep));
    const name = relPath
      .replace(FIXTURE_PREFIX, '')
      .replace(/\.mmd$/, '')
      .split('/')
      .map((segment) => slugify(segment))
      .join('__');
    return { relPath, name, text: readFileSync(file, 'utf8') };
  });
}

/**
 * The configuration keys the folder reads, from a grep of its non-spec sources (import lines
 * excluded). The list in corpus-format.ts must match it, or the corpus would miss a key.
 */
function configKeysRead(): string[] {
  const keys = new Set<string>();
  for (const file of sourceFiles(SWIMLANE_SOURCE_DIR)) {
    for (const line of readFileSync(file, 'utf8').split('\n')) {
      if (/^\s*import\b|\bfrom '/.test(line)) {
        continue;
      }
      for (const match of line.matchAll(/\bconfig\??\.([A-Z_a-z]\w*)/g)) {
        keys.add(match[1]);
      }
    }
  }
  return [...keys].sort(byText);
}

// ─────────────────────────────────────────────────────────────────────────────
// Dev server (mirrors crates/ferrigura-cose/tools/capture-corpus.ts)
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

interface PageOutcome {
  roledescription: string | null;
  pageErrors: string[];
  records: StageRecord[];
}

/** One render in a new page, or the reason it did not finish. */
async function renderInBrowser(
  browser: Browser,
  baseUrl: string,
  text: string
): Promise<PageOutcome | { skipped: string }> {
  const page = await browser.newPage({ viewport: VIEWPORT });
  const pageErrors: string[] = [];
  page.on('pageerror', (error) => pageErrors.push(error.message));
  try {
    await page.addInitScript({ content: CAPTURE_INIT_SCRIPT });
    // A large diagram makes the ?graph= URL longer than the dev server's header limit, so the
    // request is answered with e2e.html fetched without the query (the page reads location.href).
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
      return { skipped: `navigation failed: ${String(error).slice(0, 300)}` };
    }
    try {
      await page.waitForFunction(
        () => (globalThis as unknown as { rendered?: boolean }).rendered === true,
        undefined,
        { timeout: RENDER_TIMEOUT_MS }
      );
    } catch {
      return { skipped: `render timed out: no rendered flag within ${RENDER_TIMEOUT_MS} ms` };
    }
    const roledescription = await page
      .locator('svg[aria-roledescription]')
      .first()
      .getAttribute('aria-roledescription', { timeout: 5000 })
      .catch(() => null);
    const records = await page.evaluate(
      () =>
        (globalThis as unknown as { mermaidSwimlaneRecords?: StageRecord[] })
          .mermaidSwimlaneRecords ?? []
    );
    return { roledescription, pageErrors, records };
  } finally {
    await page.close();
  }
}

/** The cases and exclusions of one rendered source. */
function casesOfRender(
  name: string,
  group: CorpusCase['group'],
  source: string,
  text: string,
  outcome: PageOutcome | { skipped: string },
  keys: ReadonlySet<string>
): CaptureResult {
  if ('skipped' in outcome) {
    return { cases: [], excluded: [{ source, reason: outcome.skipped }] };
  }
  if (outcome.roledescription === null) {
    return { cases: [], excluded: [{ source, reason: 'no diagram svg on the page' }] };
  }
  if (outcome.roledescription === 'error') {
    return { cases: [], excluded: [{ source, reason: 'rendered as a syntax error diagram' }] };
  }
  if (outcome.roledescription !== 'swimlane') {
    return {
      cases: [],
      excluded: [{ source, reason: `rendered as ${outcome.roledescription}, not swimlane` }],
    };
  }
  if (outcome.pageErrors.length > 0) {
    return {
      cases: [],
      excluded: [{ source, reason: `page error: ${outcome.pageErrors[0].slice(0, 300)}` }],
    };
  }
  return casesFromCalls({
    baseName: name,
    group,
    source,
    text,
    calls: splitCalls(outcome.records),
    keys,
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// The DDLT part (capture-ddlt.spec.ts, run under vitest)
// ─────────────────────────────────────────────────────────────────────────────

function runDdltPart(): CaptureResult {
  mkdirSync(PARTS_DIR, { recursive: true });
  const partFile = join(PARTS_DIR, 'ddlt.json');
  rmSync(partFile, { force: true });
  const run = spawnSync(
    'pnpm',
    ['exec', 'vitest', 'run', '--config', join(TOOLS_DIR, 'vitest.config.ts')],
    {
      cwd: REPO_ROOT,
      env: { ...process.env, SWIMLANE_CORPUS_PARTS: PARTS_DIR },
      stdio: ['ignore', 'inherit', 'inherit'],
    }
  );
  if (run.status !== 0 || !existsSync(partFile)) {
    throw new Error(`The DDLT part failed (exit ${run.status ?? 'none'}); see the output above`);
  }
  return JSON.parse(readFileSync(partFile, 'utf8')) as CaptureResult;
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

interface Options {
  only: Set<string> | undefined;
  outDir: string;
}

function parseOptions(argv: string[]): Options {
  const value = (flag: string): string | undefined => {
    const index = argv.indexOf(flag);
    if (index < 0) {
      return undefined;
    }
    const next = argv[index + 1];
    if (next === undefined || next.startsWith('--')) {
      throw new Error(`${flag} needs a value`);
    }
    return next;
  };
  const only = value('--only');
  return {
    only: only === undefined ? undefined : new Set(only.split(',').map((name) => name.trim())),
    outDir: value('--out') ?? CORPUS_DIR,
  };
}

async function main(): Promise<void> {
  const options = parseOptions(process.argv.slice(2));
  const wanted = (name: string): boolean => options.only === undefined || options.only.has(name);

  const configKeys = configKeysRead();
  if (configKeys.join(',') !== [...CONFIG_KEYS].sort(byText).join(',')) {
    throw new Error(
      `The folder reads the config keys ${configKeys.join(', ')}, but corpus-format.ts lists ${CONFIG_KEYS.join(', ')}`
    );
  }
  const keySet = identifierKeySet();
  const keys = new Set(keySet);

  const cases: CorpusCase[] = [];
  const excluded: Exclusion[] = [];
  const add = (part: CaptureResult): void => {
    cases.push(...part.cases);
    excluded.push(...part.excluded);
  };

  if (options.only === undefined) {
    const ddlt = runDdltPart();
    add(ddlt);
    console.log(`DDLT part: ${ddlt.cases.length} cases, ${ddlt.excluded.length} exclusions`);
  }

  const fixtures = selectFixtures().filter((fixture) => wanted(fixture.name));
  const generated = generateSwimlanes(GENERATOR_SEED, GENERATED_COUNT).filter((diagram) =>
    wanted(`generated-${String(diagram.index).padStart(3, '0')}`)
  );
  const server = await ensureDevServer();
  const browser = await chromium.launch({ headless: true });
  try {
    for (const [i, fixture] of fixtures.entries()) {
      const outcome = await renderInBrowser(browser, server.baseUrl, fixture.text);
      const part = casesOfRender(
        fixture.name,
        'fixture',
        fixture.relPath,
        fixture.text,
        outcome,
        keys
      );
      add(part);
      console.log(
        `[fixture ${i + 1}/${fixtures.length}] ${part.cases.length > 0 ? 'ok  ' : 'skip'} ${fixture.relPath}${part.excluded.length > 0 ? `: ${part.excluded[0].reason.slice(0, 120)}` : ''}`
      );
    }
    for (const diagram of generated) {
      const name = `generated-${String(diagram.index).padStart(3, '0')}`;
      const source = `seed ${GENERATOR_SEED} index ${diagram.index}`;
      const text = `${FIXTURE_FRONT_MATTER}${diagram.text}`;
      const outcome = await renderInBrowser(browser, server.baseUrl, text);
      add(casesOfRender(name, 'generated', source, text, outcome, keys));
    }
  } finally {
    await browser.close();
    server.stop();
  }

  const totalBytes = writeCorpus(options.outDir, cases, excluded, keySet);
  const count = (group: CorpusCase['group']) => cases.filter((c) => c.group === group).length;
  console.log('');
  console.log(
    `Cases written: ${cases.length} (${count('fixture')} fixture, ${count('ddlt')} ddlt, ${count('generated')} generated) + manifest.json to ${relative(REPO_ROOT, options.outDir) || '.'}`
  );
  console.log(`Key set: ${keySet.length} identifiers; config keys: ${CONFIG_KEYS.join(', ')}`);
  console.log(`Total size: ${(totalBytes / 1_000_000).toFixed(2)} MB`);
  console.log(`Excluded: ${excluded.length}`);
  for (const entry of excluded) {
    console.log(`  ${entry.source}: ${entry.reason.slice(0, 160)}`);
  }
}

main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
