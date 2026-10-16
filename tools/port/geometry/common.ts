/**
 * What the geometry harnesses share: the reference run in Chromium, the settings both sides get,
 * the exclusions, the ferrigura invocation, the static server, JSON and images.
 *
 * Each harness keeps its own extraction of the SVG and its own pairing: `flowchart-geometry.ts`,
 * `pie-geometry.ts` and `diagram-geometry.ts`.
 *
 * The page the reference runs in holds the bundle's `mermaid`, the four faces of Lexica
 * Ultralegible (loaded before anything is measured), `renderReference` (the SVG and every ELK call
 * of the render), `geometry` (the SVG shown at scale 1 and measured by the harness's own `measure`)
 * and `sideBySide` for the images. `pageHtml` takes the harness's `measure` as source text.
 */
import { execFileSync, spawnSync } from 'node:child_process';
import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { dirname, extname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { JSON_SCHEMA, load } from 'js-yaml';
import { chromium, type Page } from 'playwright';

export const root = fileURLToPath(new URL('../../../', import.meta.url));
export const distDir = join(root, 'packages/mermaid/dist');
export const fontDir = join(root, 'crates/ferrigura-text/fonts');
export const workDir = join(root, 'target/geometry');

/**
 * Tolerance for positions and sizes, in px at scale 1 (viewBox units). Angles have their own
 * tolerance where they are compared (pie).
 */
export const PIXEL_TOLERANCE = 0.05;

/** Samples per path: at 0, 1/16, ..., 1 of its length. */
export const PATH_SAMPLES = 16;

/**
 * The ferrigura `render` example. `FERRIGURA_RENDER` replaces it with any program that takes the
 * same arguments (`--id`, the text on standard input, the SVG on standard output): the harnesses
 * are tested that way, with the reference's own SVG as the "ferrigura" output.
 */
const renderOverride = process.env.FERRIGURA_RENDER;
export const renderBinary = renderOverride ?? join(root, 'target/debug/examples/render');

/** Builds the `render` example, unless `FERRIGURA_RENDER` replaces it. */
export function buildRenderExample(): void {
  if (renderOverride !== undefined) return;
  execFileSync('cargo', ['build', '-q', '-p', 'ferrigura', '--example', 'render'], {
    cwd: root,
    stdio: 'inherit',
  });
}

// ---------------------------------------------------------------------------------------------
// Settings both sides get
// ---------------------------------------------------------------------------------------------

/** The settings of the directive every fixture gets (the brief's exact text). */
export const FORCED_SETTINGS: Record<string, unknown> = {
  htmlLabels: false,
  fontFamily: '"Lexica Ultralegible"',
  securityLevel: 'strict',
};

/** The themes ferrigura ships; any other name falls back to one of them (STATUS.md, M2). */
export const SHIPPED_THEMES: readonly string[] = ['redux-color', 'redux-dark-color'];

/**
 * The directive both sides get. A theme that ferrigura does not ship is replaced by the one it
 * falls back to (names ending in `dark` take redux-dark-color, the others redux-color), so that
 * both sides measure with the same theme sizes (`fontSize`, `radius`); colours are not compared.
 *
 * A fixture that names no theme gets redux-color too. The reference's top-level default is
 * `default`, which ferrigura does not ship; a diagram section whose own default is redux-color
 * gets the same theme either way.
 */
export function forcedDirective(theme: unknown, extra: Record<string, unknown> = {}): string {
  const settings: Record<string, unknown> = { ...FORCED_SETTINGS, ...extra };
  if (theme === undefined || (typeof theme === 'string' && !SHIPPED_THEMES.includes(theme))) {
    settings.theme =
      typeof theme === 'string' && theme.endsWith('dark') ? 'redux-dark-color' : 'redux-color';
  }
  return `%%{init: ${JSON.stringify(settings)}}%%`;
}

export const FRONT_MATTER = /^---[\t ]*\r?\n[\S\s]*?\r?\n---[\t ]*(?:\r?\n|$)/;
const INIT_DIRECTIVE = /%%{\s*(?:init|initialize)\s*:\s*([\S\s]*?)}%%/g;

/**
 * The text both sides render. The forced directive goes right after the front matter, or at the
 * top, and once more at the end. The reference merges every init directive in text order and the
 * last value wins, so a fixture's own body directive (five flowchart fixtures set htmlLabels to
 * true there) would win over the first copy; the last copy comes after it.
 */
export function patchText(source: string, extra: Record<string, unknown> = {}): string {
  const frontMatter = FRONT_MATTER.exec(source);
  const head = frontMatter ? frontMatter[0] : '';
  const body = source.slice(head.length);
  const headWithLine = head === '' || head.endsWith('\n') ? head : `${head}\n`;
  const bodyWithLine = body === '' || body.endsWith('\n') ? body : `${body}\n`;
  const directive = forcedDirective(effectiveSettings(source).theme, extra);
  return `${headWithLine}${directive}\n${bodyWithLine}${directive}\n`;
}

/**
 * The look, layout and theme the fixture's text asks for: front matter first, then the directives
 * in order, as the reference merges them (a later key wins).
 */
export function effectiveSettings(source: string): {
  look: unknown;
  layout: unknown;
  theme: unknown;
} {
  const settings: Record<string, unknown> = {};
  const frontMatter = FRONT_MATTER.exec(source);
  if (frontMatter) {
    const body = frontMatter[0].replace(/^---[^\n]*\n/, '').replace(/\n---[^\n]*\n?$/, '');
    const metadata = load(body, { schema: JSON_SCHEMA }) as
      | { config?: Record<string, unknown> }
      | undefined;
    Object.assign(settings, metadata?.config ?? {});
  }
  for (const match of source.matchAll(INIT_DIRECTIVE)) {
    try {
      const args = JSON.parse(match[1].replace(/'/g, '"')) as Record<string, unknown>;
      Object.assign(settings, args);
    } catch {
      // A directive that is not JSON is not read by the reference either.
    }
  }
  return { look: settings.look, layout: settings.layout, theme: settings.theme };
}

// ---------------------------------------------------------------------------------------------
// Exclusions
// ---------------------------------------------------------------------------------------------

/**
 * Layouts that are not layered. ferrigura implements `elk.layered` only and falls back to it for
 * the others, so a fixture that names one of these is excluded from the comparison.
 */
export const NON_LAYERED_LAYOUTS: readonly string[] = [
  'elk.box',
  'elk.rectpacking',
  'elk.stress',
  'elk.force',
  'elk.mrtree',
  'elk.sporeOverlap',
  'cose-bilkent',
  'swimlane',
];

/** The look that is out of scope: rough.js sketching is not part of the port. */
export const EXCLUDED_LOOK = 'handDrawn';

/** Why a fixture is excluded from the comparison, or null when it is compared. */
export function exclusionReason(source: string): string | null {
  const { look, layout } = effectiveSettings(source);
  if (look === EXCLUDED_LOOK) {
    return `look ${EXCLUDED_LOOK}: rough.js sketching is out of scope`;
  }
  if (typeof layout === 'string' && NON_LAYERED_LAYOUTS.includes(layout)) {
    return `layout ${layout}: not layered, ferrigura falls back to layered`;
  }
  // Scripts the bundled faces do not cover need a fallback font, which ferrigura does not have
  // yet (STATUS.md, open since M2); the reference draws them in a system font.
  if (/[⺀-鿿가-힯豈-﫿]/.test(source)) {
    return 'CJK text: needs a fallback font, which ferrigura does not have yet';
  }
  // A subgraph's own algorithm, from its shape data (`grp@{ algorithm: elk.box }`).
  for (const match of source.matchAll(/algorithm\s*:\s*["']?([\w.-]+)/g)) {
    if (NON_LAYERED_LAYOUTS.includes(match[1])) {
      return `subgraph algorithm ${match[1]}: not layered, ferrigura falls back to layered`;
    }
  }
  return null;
}

// ---------------------------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------------------------

/**
 * The class of a fixture: compared and within or outside the tolerance, not rendered by ferrigura,
 * excluded with a reason, or failed in the reference (which makes it a fault of the harness).
 */
export type FixtureClass = 'within' | 'outside' | 'not rendered' | 'excluded' | 'reference failed';

/** The fields every harness reports per fixture. */
export interface FixtureResult {
  fixture: string;
  id: string;
  class: FixtureClass;
  /** The exclusion reason, the ferrigura error or the reference error. */
  reason: string | null;
  worstPercent: number | null;
  problems: string[];
  warnings: string[];
}

/** The number of results in each class. */
export function countClasses(
  results: readonly { class: FixtureClass }[]
): Record<FixtureClass, number> {
  const counts: Record<FixtureClass, number> = {
    within: 0,
    outside: 0,
    'not rendered': 0,
    excluded: 0,
    'reference failed': 0,
  };
  for (const result of results) counts[result.class]++;
  return counts;
}

// ---------------------------------------------------------------------------------------------
// Files and command line
// ---------------------------------------------------------------------------------------------

export async function writeJson(path: string, value: unknown): Promise<void> {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, JSON.stringify(value, null, 2));
}

/** Every `.mmd` file under a directory, at any depth, as paths relative to it, sorted. */
export async function mmdFiles(directory: string, prefix = ''): Promise<string[]> {
  const found: string[] = [];
  for (const entry of await readdir(join(directory, prefix), { withFileTypes: true })) {
    const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isDirectory()) found.push(...(await mmdFiles(directory, relative)));
    else if (extname(entry.name) === '.mmd') found.push(relative);
  }
  return found.sort();
}

/** The id of a fixture's diagram: `ref-` and the path without `.mmd`, with other characters as `-`. */
export function slugOf(relativePath: string): string {
  return relativePath.replace(/\.mmd$/, '').replace(/[^\w-]+/g, '-');
}

/** Substring match, or a glob with `*` (any characters but `/`), `**` (any) and `?`. */
export function matchesFilter(relativePath: string, filter: string): boolean {
  if (!/[*?]/.test(filter)) return relativePath.includes(filter);
  const source = filter
    .replace(/[$()+.[\\\]^{|}]/g, '\\$&')
    .replaceAll('**', '@@ANY@@')
    .replaceAll('*', '[^/]*')
    .replaceAll('?', '[^/]')
    .replaceAll('@@ANY@@', '.*');
  return new RegExp(`^${source}$`).test(relativePath);
}

/** The value after a flag, or null when the flag is absent. A flag without a value is an error. */
export function flagValue(argv: string[], flag: string): string | null {
  const index = argv.indexOf(flag);
  if (index < 0) return null;
  const next = argv[index + 1];
  if (next === undefined || next.startsWith('--')) throw new Error(`${flag} needs a value`);
  return next;
}

// ---------------------------------------------------------------------------------------------
// The ferrigura invocation
// ---------------------------------------------------------------------------------------------

/** The ferrigura render of the text, or the first line of its error. */
/**
 * Ferrigura's SVG of the text. `now` is the clock the reference was given, in milliseconds since
 * the epoch, so that the dates that depend on it (the gantt today marker) agree.
 */
export function renderFerrigura(
  text: string,
  id: string,
  now?: number,
  seed?: number
): { svg: string } | { error: string } {
  const args = ['--id', id];
  if (now !== undefined) args.push('--now', String(Math.trunc(now)));
  // The seed of the reference's random numbers: the generated gitGraph ids (commits, merges and
  // cherry-picks without an `id:`) are drawn from the same generator.
  if (seed !== undefined) args.push('--seed', String(seed));
  const result = spawnSync(renderBinary, args, {
    input: text,
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error) return { error: result.error.message };
  if (result.status !== 0) {
    const line = (result.stderr || `exit status ${result.status}`).trim().split('\n')[0];
    // A panic names its thread id, which would keep equal causes apart in the summary.
    return { error: line.replace(/^thread '([^']*)' \(\d+\) /, "thread '$1' ") };
  }
  return { svg: result.stdout };
}

// ---------------------------------------------------------------------------------------------
// The page: the reference and the measurement, in Chromium
// ---------------------------------------------------------------------------------------------

/** What the page returns for a reference render: the SVG and the ELK calls, in order. */
export interface ReferenceRender {
  ok: boolean;
  svg: string | null;
  error: string | null;
  elk: { input: JsonValue; output: JsonValue }[];
}

export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

/**
 * The page. `measure` is the harness's measurement of one SVG, as source text: it defines
 * `const measure = (svg) => ...`, using the helpers of this page (`numbers`, `mapPoint`,
 * `userMatrix`, `boxOf`), which map every box into the viewBox coordinates of the root svg.
 */
export function pageHtml(measure: string): string {
  return `<!doctype html>
<html><head><meta charset="utf-8">
<style>
@font-face { font-family: "Lexica Ultralegible"; src: url(/fonts/LexicaUltralegible-Regular.ttf) format("truetype"); font-weight: 400; font-style: normal; }
@font-face { font-family: "Lexica Ultralegible"; src: url(/fonts/LexicaUltralegible-Bold.ttf) format("truetype"); font-weight: 700; font-style: normal; }
@font-face { font-family: "Lexica Ultralegible"; src: url(/fonts/LexicaUltralegible-Italic.ttf) format("truetype"); font-weight: 400; font-style: italic; }
@font-face { font-family: "Lexica Ultralegible"; src: url(/fonts/LexicaUltralegible-BoldItalic.ttf) format("truetype"); font-weight: 700; font-style: italic; }
body { margin: 0; background: #ffffff; }
#stage { width: 1000px; }
</style>
<script>
// Installed before mermaid loads. render.ts calls the hook after every ELK layout with the input
// (copied before layout) and the laid-out graph.
window.__elkCalls = [];
window.__mermaidElkCapture = function (input, output) {
  window.__elkCalls.push({ input: input, output: JSON.parse(JSON.stringify(output)) });
};
</script>
</head>
<body><div id="stage"></div>
<script type="module">
import mermaid from '/dist/mermaid.esm.mjs';

// The four faces must be loaded before mermaid measures text, or it measures a fallback face.
await Promise.all([
  '400 16px "Lexica Ultralegible"',
  'italic 400 16px "Lexica Ultralegible"',
  'bold 16px "Lexica Ultralegible"',
  'italic bold 16px "Lexica Ultralegible"',
].map((face) => document.fonts.load(face)));
await document.fonts.ready;

const numbers = (text) => (text || '').trim().split(/[\\s,]+/).filter(Boolean).map(Number);
const mapPoint = (svg, matrix, x, y) => {
  const point = svg.createSVGPoint();
  point.x = x;
  point.y = y;
  const mapped = point.matrixTransform(matrix);
  return { x: mapped.x, y: mapped.y };
};
// The matrix from an element's user units to the user units of the root svg (the viewBox). The
// root's own getCTM includes the viewBox transform and the display scale, so multiplying by its
// inverse leaves the viewBox coordinates whatever size the svg is shown at.
const userMatrix = (svg, element) => {
  const inner = element.getCTM();
  const outer = svg.getCTM();
  return inner && outer ? outer.inverse().multiply(inner) : null;
};
// The box of an element, in the viewBox coordinates.
const boxOf = (svg, element) => {
  const matrix = userMatrix(svg, element);
  if (!matrix) return null;
  const box = element.getBBox();
  const corners = [
    mapPoint(svg, matrix, box.x, box.y),
    mapPoint(svg, matrix, box.x + box.width, box.y),
    mapPoint(svg, matrix, box.x, box.y + box.height),
    mapPoint(svg, matrix, box.x + box.width, box.y + box.height),
  ];
  const xs = corners.map((c) => c.x);
  const ys = corners.map((c) => c.y);
  const x = Math.min(...xs);
  const y = Math.min(...ys);
  return { x, y, width: Math.max(...xs) - x, height: Math.max(...ys) - y };
};

${measure}

// The clock and the random numbers of a render are fixed when a clock is given: the today marker of
// a gantt and the generated ids (gitGraph commits, rough.js strokes) read them, and two renders of
// the same text would otherwise differ. Both get the same time and seed, so they are equal.
window.renderReference = async function (id, text, clock) {
  mermaid.initialize({ startOnLoad: false });
  window.__elkCalls = [];
  const RealDate = Date;
  const realRandom = Math.random;
  if (clock) {
    const fixed = clock.now;
    window.Date = class extends RealDate {
      constructor(...args) {
        if (args.length === 0) super(fixed);
        else super(...args);
      }
      static now() {
        return fixed;
      }
    };
    let state = clock.seed >>> 0;
    Math.random = function () {
      state = (state + 0x6d2b79f5) | 0;
      let t = Math.imul(state ^ (state >>> 15), 1 | state);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }
  try {
    const result = await mermaid.render(id, text);
    return { ok: true, svg: result.svg, error: null, elk: window.__elkCalls };
  } catch (error) {
    return { ok: false, svg: null, error: String((error && error.message) || error), elk: window.__elkCalls };
  } finally {
    window.Date = RealDate;
    Math.random = realRandom;
  }
};

window.geometry = async function (svgText) {
  const stage = document.getElementById('stage');
  stage.innerHTML = svgText;
  const svg = stage.querySelector('svg');
  // Show the diagram at scale 1, so Chromium lays the text out at the size it was measured at:
  // a scaled svg (width 100%, a fractional max-width) gets its text laid out at the scaled size,
  // which moves label boxes by a few 1/64 px. The viewBox keeps its origin and grows to whole
  // pixels, so user coordinates are unchanged; the original viewBox is what is compared.
  const original = svg.getAttribute('viewBox');
  const attributes = {
    width: svg.getAttribute('width'),
    height: svg.getAttribute('height'),
    style: svg.getAttribute('style'),
  };
  const [x, y, w, h] = numbers(original);
  if ([x, y, w, h].every(Number.isFinite)) {
    const width = Math.ceil(w);
    const height = Math.ceil(h);
    svg.setAttribute('viewBox', x + ' ' + y + ' ' + width + ' ' + height);
    svg.setAttribute('width', String(width));
    svg.setAttribute('height', String(height));
    svg.style.maxWidth = 'none';
  }
  await document.fonts.ready;
  return { ...measure(svg), ...attributes, viewBox: numbers(original) };
};

window.sideBySide = function (left, right) {
  const stage = document.getElementById('stage');
  stage.innerHTML = '';
  const row = document.createElement('div');
  row.style.cssText = 'display:flex;gap:24px;align-items:flex-start;padding:16px;background:#ffffff;width:max-content;';
  for (const svgText of [left, right]) {
    const cell = document.createElement('div');
    cell.style.cssText = 'width:560px;';
    cell.innerHTML = svgText;
    const svg = cell.querySelector('svg');
    svg.removeAttribute('height');
    svg.setAttribute('width', '560');
    svg.style.maxWidth = 'none';
    row.appendChild(cell);
  }
  stage.appendChild(row);
};

window.ready = true;
</script></body></html>`;
}

const CONTENT_TYPES: Record<string, string> = {
  '.ttf': 'font/ttf',
  '.css': 'text/css',
  '.json': 'application/json',
  '.wasm': 'application/wasm',
};

/** A static server for the page, the bundle and the fonts, on a free port of 127.0.0.1. */
export function serve(html: string): Promise<{ server: Server; port: number }> {
  const handle = async (request: IncomingMessage, response: ServerResponse): Promise<void> => {
    const url = new URL(request.url ?? '/', 'http://localhost');
    try {
      if (url.pathname === '/') {
        response.writeHead(200, { 'content-type': 'text/html' }).end(html);
        return;
      }
      const mapping: [string, string][] = [
        ['/dist/', distDir],
        ['/fonts/', fontDir],
      ];
      const match = mapping.find(([prefix]) => url.pathname.startsWith(prefix));
      if (!match) {
        response.writeHead(404).end();
        return;
      }
      const file = join(match[1], decodeURIComponent(url.pathname.slice(match[0].length)));
      const body = await readFile(file);
      const type = CONTENT_TYPES[extname(file)] ?? 'text/javascript';
      response.writeHead(200, { 'content-type': type }).end(body);
    } catch {
      response.writeHead(404).end();
    }
  };
  const server = createServer((request, response) => {
    void handle(request, response);
  });
  return new Promise((resolve) => {
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      resolve({ server, port: typeof address === 'object' && address ? address.port : 0 });
    });
  });
}

/**
 * Runs `run` on a page of the harness: Chromium with the bundle, the fonts and `measure`, all
 * loaded from the local server. Chromium is started with `--font-render-hinting=none`, as the other
 * geometry checks are: without it, Chromium keeps fractional glyph advances only on macOS.
 */
export async function withPage<T>(measure: string, run: (page: Page) => Promise<T>): Promise<T> {
  await mkdir(workDir, { recursive: true });
  const { server, port } = await serve(pageHtml(measure));
  const browser = await chromium.launch({ args: ['--font-render-hinting=none'] });
  try {
    // The reference reads dates in the browser's time zone and ferrigura in UTC (M6-PLAN, gantt
    // time), so the page runs in UTC.
    const page = await browser.newPage({
      viewport: { width: 1200, height: 900 },
      timezoneId: 'UTC',
    });
    page.on('pageerror', (error) => console.error('page error:', error.message));
    await page.goto(`http://127.0.0.1:${port}/`);
    await page.waitForFunction(
      () => (window as unknown as { ready?: boolean }).ready === true,
      null,
      { timeout: 60_000 }
    );
    return await run(page);
  } finally {
    await browser.close();
    server.close();
  }
}

/** The reference's render of the text, with its SVG and ELK calls. */
/**
 * The clock and the seed of a render: the time the render sees and the seed of its random numbers.
 * The same clock for two renders makes them equal (the self check of the generic harness).
 */
export interface RenderClock {
  now: number;
  seed: number;
}

export function renderReference(
  page: Page,
  id: string,
  text: string,
  clock?: RenderClock
): Promise<ReferenceRender> {
  return page.evaluate(
    ([svgId, source, fixed]) =>
      (
        window as unknown as {
          renderReference: (
            id: string,
            text: string,
            clock?: RenderClock
          ) => Promise<ReferenceRender>;
        }
      ).renderReference(svgId, source, fixed ?? undefined),
    [id, text, clock ?? null] as const
  ) as Promise<ReferenceRender>;
}

/** The page's measurement of one SVG at scale 1. The page's own `measure` returns its shape. */
export function measureSvg<T>(page: Page, svg: string): Promise<T> {
  return page.evaluate(
    (text) => (window as unknown as { geometry: (svg: string) => Promise<unknown> }).geometry(text),
    svg
  ) as Promise<T>;
}

/** Writes the reference and the ferrigura SVG side by side, as a PNG at `file`. */
export async function writeSideBySide(
  page: Page,
  reference: string,
  rendered: string,
  file: string
): Promise<void> {
  await page.evaluate(
    ([left, right]) =>
      (window as unknown as { sideBySide: (a: string, b: string) => void }).sideBySide(left, right),
    [reference, rendered] as const
  );
  await mkdir(dirname(file), { recursive: true });
  await page.locator('#stage > div').screenshot({ path: file });
}
