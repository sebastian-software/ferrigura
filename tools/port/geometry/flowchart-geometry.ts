/**
 * Geometry comparison of flowcharts: the JavaScript reference and ferrigura, for the fixtures
 * under `e2e/diagrams/flowchart` (all of them, or those picked by `--filter` and `--stage`).
 *
 * Both sides get the same text. The text is patched with the directive `%%{init}%%` that forces
 * HTML labels off, the font to Lexica Ultralegible and the security level to strict. It goes right
 * after the front matter, where it overrides the fixture's own `htmlLabels` and `fontFamily`
 * (front matter is merged before directives), and once more at the end of the text, because the
 * reference takes the last value of each key over all directives and five fixtures set
 * `htmlLabels: true` in a directive in their body. The run also counts `foreignObject` elements in
 * the reference, which must be zero. Look and theme are not forced: the flowchart defaults (look
 * neo, theme redux-color) apply on both sides, as they do for a fixture that configures nothing.
 *
 * The reference is Mermaid's built bundle (`packages/mermaid/dist`), run in Chromium. ferrigura
 * renders the same patched text with its `render` example, with `--id` set to the reference's
 * id, so both SVGs can be paired by element id. Both SVGs are measured in one Chromium page, in
 * their viewBox coordinates: the nodes, the edge paths (sampled), the edge labels, the clusters,
 * the root box and the element counts per class.
 *
 * Excluded fixtures are a class of their own, not failures: the effective text sets look
 * `handDrawn` (rough.js is out of scope), or a layout that is not layered (ferrigura falls back to
 * layered). A fixture that the reference renders and ferrigura does not is "not rendered": the
 * `render` example fails for every flowchart until ferrigura draws one.
 *
 * With `--capture`, the reference's SVG, its geometry and every ELK call of its render (input and
 * output, in order) are written under `target/geometry/flowchart/<slug>/`. The ELK input is the
 * reference data for the Rust side. If `ferrigura --example flowchart-elk` builds, its `elkInput`
 * is compared with the reference's input, in node and edge order, sizes, labels and layout options.
 *
 * Tolerance: 0.05 px for positions and sizes, as in the pie harness; counts and marker kinds must
 * match exactly. The worst difference is reported as a percentage of the tolerance.
 *
 * Usage:
 *   pnpm port:geometry-flowchart [--filter PATTERN] [--stage NAME[,NAME...]] [--images DIR] [--capture]
 *   DUMP=SLUG[,SLUG...] pnpm port:geometry-flowchart   writes target/geometry/dump-SLUG.json
 *
 * `FERRIGURA_RENDER` and `FERRIGURA_FLOWCHART_ELK` replace the ferrigura binaries (any program with
 * the same arguments); they exist to test the harness itself.
 *
 * Exit code 1 only if a fixture that rendered on both sides is outside the tolerance.
 */
import { execFileSync, spawnSync } from 'node:child_process';
import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { extname, join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import { JSON_SCHEMA, load } from 'js-yaml';
import { chromium, type Page } from 'playwright';

import {
  compareElkInput,
  compareGeometry,
  PIXEL_TOLERANCE,
  type ElkElement,
  type SvgGeometry,
} from './flowchart-compare.js';
import { EXCLUDED_LOOK, NON_LAYERED_LAYOUTS, STAGES } from './flowchart-stages.js';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const fixtureDir = join(root, 'e2e/diagrams/flowchart');
const distDir = join(root, 'packages/mermaid/dist');
const fontDir = join(root, 'crates/ferrigura-text/fonts');
const workDir = join(root, 'target/geometry');
/**
 * The ferrigura binaries. `FERRIGURA_RENDER` and `FERRIGURA_FLOWCHART_ELK` replace them with any
 * program that takes the same arguments: the harness is tested that way, with the reference's own
 * SVGs as the "ferrigura" output, before ferrigura draws flowcharts.
 */
const renderOverride = process.env.FERRIGURA_RENDER;
const elkOverride = process.env.FERRIGURA_FLOWCHART_ELK;
const renderBinary = renderOverride ?? join(root, 'target/debug/examples/render');
const elkBinary = elkOverride ?? join(root, 'target/debug/examples/flowchart-elk');

/** Samples per edge path: at 0, 1/16, ..., 1 of its length. */
const SAMPLES = 16;

/** The directive every fixture gets right after its front matter (the brief's exact text). */
const FORCED_SETTINGS: Record<string, unknown> = {
  htmlLabels: false,
  fontFamily: '"Lexica Ultralegible"',
  securityLevel: 'strict',
};

/** The themes ferrigura ships; any other name falls back to one of them (STATUS.md, M2). */
const SHIPPED_THEMES = ['redux-color', 'redux-dark-color'];

/**
 * The directive both sides get. A theme that ferrigura does not ship is replaced by the one it
 * falls back to (names ending in `dark` take redux-dark-color, the others redux-color), so that
 * both sides measure with the same theme sizes (`fontSize`, `radius`); colours are not compared.
 */
function forcedDirective(theme: unknown): string {
  const settings = { ...FORCED_SETTINGS };
  if (typeof theme === 'string' && !SHIPPED_THEMES.includes(theme)) {
    settings.theme = theme.endsWith('dark') ? 'redux-dark-color' : 'redux-color';
  }
  return `%%{init: ${JSON.stringify(settings)}}%%`;
}

const FRONT_MATTER = /^---[\t ]*\r?\n[\S\s]*?\r?\n---[\t ]*(?:\r?\n|$)/;
const INIT_DIRECTIVE = /%%{\s*(?:init|initialize)\s*:\s*([\S\s]*?)}%%/g;

/** What the page returns for a reference render: the SVG and the ELK calls, in order. */
interface ReferenceRender {
  ok: boolean;
  svg: string | null;
  error: string | null;
  elk: { input: JsonValue; output: JsonValue }[];
}

type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };

// ---------------------------------------------------------------------------------------------
// The page: renders the reference and measures both SVGs in the page's coordinates
// ---------------------------------------------------------------------------------------------

const harness = `<!doctype html>
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
const SHAPES = 'rect, circle, ellipse, polygon, path';
const notInLabel = (element) => element.closest('g.label') === null;

const measure = (svg) => {
  // Nodes: one group per vertex, the centre is the group's origin in viewBox coordinates.
  const nodes = [...svg.querySelectorAll('g.nodes > g')].map((group) => {
    const labelShape = group.querySelector('.label-container');
    const fallback = [...group.querySelectorAll(SHAPES)].find(notInLabel) || null;
    const shape = labelShape || fallback;
    const label = group.querySelector('g.label');
    const groupMatrix = userMatrix(svg, group);
    return {
      id: group.id,
      transform: group.getAttribute('transform'),
      centre: groupMatrix ? mapPoint(svg, groupMatrix, 0, 0) : null,
      shape: {
        source: labelShape ? 'label-container' : fallback ? 'shape' : 'none',
        tag: shape ? shape.tagName : null,
        box: shape ? boxOf(svg, shape) : null,
      },
      label: label ? boxOf(svg, label) : null,
    };
  });
  const edges = [...svg.querySelectorAll('g.edgePaths > path')].map((path) => {
    const total = path.getTotalLength();
    const matrix = userMatrix(svg, path);
    const samples = [];
    for (let index = 0; index <= ${SAMPLES}; index++) {
      const point = path.getPointAtLength((total * index) / ${SAMPLES});
      samples.push(mapPoint(svg, matrix, point.x, point.y));
    }
    const raw = path.getAttribute('data-points');
    let dataPoints = null;
    if (raw !== null) {
      try {
        dataPoints = JSON.parse(atob(raw)).map((point) => ({ x: point.x, y: point.y }));
      } catch {
        dataPoints = 'invalid';
      }
    }
    return {
      id: path.id,
      d: path.getAttribute('d'),
      length: total,
      samples,
      dataPoints,
      markerStart: path.getAttribute('marker-start'),
      markerEnd: path.getAttribute('marker-end'),
    };
  });
  const edgeLabels = [...svg.querySelectorAll('g.label[data-id]')].map((label) => {
    const background = label.querySelector('rect.background');
    return {
      dataId: label.getAttribute('data-id'),
      box: boxOf(svg, label),
      background: background ? boxOf(svg, background) : null,
    };
  });
  const clusters = [...svg.querySelectorAll('g.clusters > g.cluster')].map((group) => {
    const rect = group.querySelector(':scope > rect') || group.querySelector('rect');
    const label = group.querySelector(':scope > g.cluster-label') || group.querySelector('g.cluster-label');
    return {
      id: group.id,
      rect: rect ? boxOf(svg, rect) : null,
      label: label ? boxOf(svg, label) : null,
    };
  });
  // Elements per class token: a structural check (counts must match).
  const counts = {};
  for (const element of svg.querySelectorAll('*')) {
    const classes = element.getAttribute('class');
    if (!classes) continue;
    for (const token of classes.split(/\\s+/)) {
      if (token) counts[token] = (counts[token] || 0) + 1;
    }
  }
  const looks = [...svg.querySelectorAll('[data-look]')]
    .map((element) => element.getAttribute('data-look'))
    .filter((look, index, all) => all.indexOf(look) === index)
    .sort();
  return {
    id: svg.id,
    viewBox: numbers(svg.getAttribute('viewBox')),
    width: svg.getAttribute('width'),
    height: svg.getAttribute('height'),
    style: svg.getAttribute('style'),
    roledescription: svg.getAttribute('aria-roledescription'),
    looks,
    foreignObjects: svg.querySelectorAll('foreignObject').length,
    nodes,
    edges,
    edgeLabels,
    clusters,
    counts,
  };
};

window.renderReference = async function (id, text) {
  mermaid.initialize({ startOnLoad: false });
  window.__elkCalls = [];
  try {
    const result = await mermaid.render(id, text);
    return { ok: true, svg: result.svg, error: null, elk: window.__elkCalls };
  } catch (error) {
    return { ok: false, svg: null, error: String((error && error.message) || error), elk: window.__elkCalls };
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

// ---------------------------------------------------------------------------------------------
// Fixtures, the patched text and the exclusions
// ---------------------------------------------------------------------------------------------

/** Every `.mmd` file under a directory, at any depth, as paths relative to it. */
async function mmdFiles(directory: string, prefix = ''): Promise<string[]> {
  const found: string[] = [];
  for (const entry of await readdir(join(directory, prefix), { withFileTypes: true })) {
    const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isDirectory()) found.push(...(await mmdFiles(directory, relative)));
    else if (extname(entry.name) === '.mmd') found.push(relative);
  }
  return found;
}

/** The id of a fixture's diagram: `ref-` and the path without `.mmd`, with other characters as `-`. */
function slugOf(relativePath: string): string {
  return relativePath.replace(/\.mmd$/, '').replace(/[^\w-]+/g, '-');
}

/**
 * The text both sides render. The forced directive goes right after the front matter, or at the
 * top, and once more at the end. The reference merges every init directive in text order and the
 * last value wins, so a fixture's own body directive (five fixtures set `htmlLabels: true` there)
 * would win over the first copy; the last copy comes after it.
 */
function patchText(source: string): string {
  const frontMatter = FRONT_MATTER.exec(source);
  const head = frontMatter ? frontMatter[0] : '';
  const body = source.slice(head.length);
  const headWithLine = head === '' || head.endsWith('\n') ? head : `${head}\n`;
  const bodyWithLine = body === '' || body.endsWith('\n') ? body : `${body}\n`;
  const directive = forcedDirective(effectiveSettings(source).theme);
  return `${headWithLine}${directive}\n${bodyWithLine}${directive}\n`;
}

/**
 * The look and layout the fixture's text asks for: front matter first, then the directives in
 * order, as the reference merges them (a later key wins).
 */
function effectiveSettings(source: string): { look: unknown; layout: unknown; theme: unknown } {
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

/** Why a fixture is excluded from the comparison, or null when it is compared. */
function exclusionReason(source: string): string | null {
  const { look, layout } = effectiveSettings(source);
  if (look === EXCLUDED_LOOK) {
    return `look ${EXCLUDED_LOOK}: rough.js sketching is out of scope`;
  }
  if (typeof layout === 'string' && NON_LAYERED_LAYOUTS.includes(layout)) {
    return `layout ${layout}: not layered, ferrigura falls back to layered`;
  }
  // Scripts the bundled faces do not cover need a fallback font, which ferrigura does not have
  // yet (STATUS.md, open since M2); the reference draws them in a system font.
  if (/[\u2E80-\u9FFF\uAC00-\uD7AF\uF900-\uFAFF]/.test(source)) {
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
// The run
// ---------------------------------------------------------------------------------------------

type FixtureClass = 'within' | 'outside' | 'not rendered' | 'excluded' | 'reference failed';

interface FixtureResult {
  fixture: string;
  id: string;
  class: FixtureClass;
  /** The exclusion reason, the ferrigura error or the reference error. */
  reason: string | null;
  worstPercent: number | null;
  problems: string[];
  warnings: string[];
  elk: {
    status: 'not available' | 'not compared' | 'match' | 'differ';
    referenceCalls: number;
    problems: string[];
  };
}

interface Options {
  filter: string | null;
  stages: string[];
  imageDir: string | null;
  capture: boolean;
}

function parseOptions(argv: string[]): Options {
  const value = (flag: string): string | null => {
    const index = argv.indexOf(flag);
    if (index < 0) return null;
    const next = argv[index + 1];
    if (next === undefined || next.startsWith('--')) throw new Error(`${flag} needs a value`);
    return next;
  };
  const stage = value('--stage');
  const stages = stage
    ? stage
        .split(',')
        .map((name) => name.trim())
        .filter(Boolean)
    : [];
  for (const name of stages) {
    if (!(name in STAGES)) {
      throw new Error(`unknown stage "${name}" (stages: ${Object.keys(STAGES).join(', ')})`);
    }
  }
  return {
    filter: value('--filter'),
    stages,
    imageDir: value('--images'),
    capture: argv.includes('--capture'),
  };
}

/** Substring match, or a glob with `*` (any characters but `/`), `**` (any) and `?`. */
function matchesFilter(relativePath: string, filter: string): boolean {
  if (!/[*?]/.test(filter)) return relativePath.includes(filter);
  const source = filter
    .replace(/[$()+.[\\\]^{|}]/g, '\\$&')
    .replaceAll('**', '@@ANY@@')
    .replaceAll('*', '[^/]*')
    .replaceAll('?', '[^/]')
    .replaceAll('@@ANY@@', '.*');
  return new RegExp(`^${source}$`).test(relativePath);
}

/** A static server for the harness, the bundle and the fonts. */
function serve(): Promise<{ server: Server; port: number }> {
  const handle = async (request: IncomingMessage, response: ServerResponse): Promise<void> => {
    const url = new URL(request.url ?? '/', 'http://localhost');
    try {
      if (url.pathname === '/') {
        response.writeHead(200, { 'content-type': 'text/html' }).end(harness);
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
      const type = extname(file) === '.ttf' ? 'font/ttf' : 'text/javascript';
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

/** The ferrigura render of the patched text, or the first line of its error. */
function renderFerrigura(text: string, id: string): { svg: string } | { error: string } {
  const result = spawnSync(renderBinary, ['--id', id], {
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

/** The `flowchart-elk` example's ELK input for the text, or the first line of its error. */
function runElkExample(text: string, id: string): { input: ElkElement } | { error: string } {
  const result = spawnSync(elkBinary, ['--id', id], {
    input: text,
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error) return { error: result.error.message };
  if (result.status !== 0) {
    return { error: (result.stderr || `exit status ${result.status}`).trim().split('\n')[0] };
  }
  try {
    return { input: (JSON.parse(result.stdout) as { elkInput: ElkElement }).elkInput };
  } catch (error) {
    return { error: `output is not JSON: ${(error as Error).message}` };
  }
}

/** Whether the `flowchart-elk` example builds. Reported once if it does not. */
function elkExampleBuilds(): string | null {
  if (elkOverride !== undefined) return null;
  try {
    execFileSync('cargo', ['build', '-q', '-p', 'ferrigura', '--example', 'flowchart-elk'], {
      cwd: root,
      stdio: ['ignore', 'ignore', 'pipe'],
    });
    return null;
  } catch (error) {
    const stderr = (error as { stderr?: Buffer }).stderr?.toString() ?? '';
    return (
      stderr
        .trim()
        .split('\n')
        .find((line) => line.trim() !== '') ?? 'does not build'
    );
  }
}

async function writeJson(path: string, value: unknown): Promise<void> {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, JSON.stringify(value, null, 2));
}

/** The page's measurement of one SVG. It puts the SVG on the stage first. */
async function measure(page: Page, svg: string): Promise<SvgGeometry> {
  return (await page.evaluate(
    (text) =>
      (window as unknown as { geometry: (svg: string) => Promise<SvgGeometry> }).geometry(text),
    svg
  )) as SvgGeometry;
}

async function main(): Promise<void> {
  const options = parseOptions(process.argv.slice(2));
  // DUMP=<slug>[,<slug>...] writes target/geometry/dump-<slug>.json for each fixture named.
  const dumpSlugs = new Set((process.env.DUMP ?? '').split(',').filter(Boolean));

  if (renderOverride === undefined) {
    execFileSync('cargo', ['build', '-q', '-p', 'ferrigura', '--example', 'render'], {
      cwd: root,
      stdio: 'inherit',
    });
  }
  const elkProblem = elkExampleBuilds();
  if (elkProblem !== null) {
    console.log(
      `ELK stage input: stages not available (ferrigura --example flowchart-elk: ${elkProblem})`
    );
  }

  // Fixtures, keyed by their path relative to the flowchart folder.
  const all = (await mmdFiles(fixtureDir)).sort();
  const slugs = new Map<string, string>();
  for (const file of all) {
    const slug = slugOf(file);
    if (slugs.has(slug))
      throw new Error(`fixtures ${slugs.get(slug)} and ${file} share the id ${slug}`);
    slugs.set(slug, file);
  }
  const inStages = new Set<string>();
  for (const name of options.stages) {
    for (const file of STAGES[name]) {
      if (!all.includes(file)) throw new Error(`stage ${name} lists ${file}, which does not exist`);
      inStages.add(file);
    }
  }
  const selected = all.filter(
    (file) =>
      (options.stages.length === 0 || inStages.has(file)) &&
      (options.filter === null || matchesFilter(file, options.filter))
  );
  console.log(`flowchart geometry: ${selected.length} of ${all.length} fixtures selected`);

  await mkdir(workDir, { recursive: true });
  const { server, port } = await serve();
  const browser = await chromium.launch({
    // Without hinting, Chromium keeps fractional glyph advances, as it does on macOS.
    args: ['--font-render-hinting=none'],
  });
  const results: FixtureResult[] = [];
  let foreignObjectFixtures = 0;
  try {
    const page: Page = await browser.newPage({ viewport: { width: 1200, height: 900 } });
    page.on('pageerror', (error) => console.error('page error:', error.message));
    await page.goto(`http://127.0.0.1:${port}/`);
    await page.waitForFunction(
      () => (window as unknown as { ready?: boolean }).ready === true,
      null,
      { timeout: 60_000 }
    );

    for (const file of selected) {
      const slug = slugOf(file);
      const id = `ref-${slug}`;
      const fixtureSource = await readFile(join(fixtureDir, file), 'utf8');
      const text = patchText(fixtureSource);
      const result: FixtureResult = {
        fixture: file,
        id,
        class: 'reference failed',
        reason: null,
        worstPercent: null,
        problems: [],
        warnings: [],
        elk: {
          status: elkProblem === null ? 'not compared' : 'not available',
          referenceCalls: 0,
          problems: [],
        },
      };
      results.push(result);

      const reference = (await page.evaluate(
        ([svgId, patched]) =>
          (
            window as unknown as {
              renderReference: (id: string, text: string) => Promise<ReferenceRender>;
            }
          ).renderReference(svgId, patched),
        [id, text] as const
      )) as ReferenceRender;
      result.elk.referenceCalls = reference.elk.length;
      if (!reference.ok || reference.svg === null) {
        result.reason = reference.error;
        console.log(`! ${file}  reference failed: ${reference.error}`);
        continue;
      }
      const referenceGeometry = await measure(page, reference.svg);
      if (referenceGeometry.roledescription !== 'flowchart-v2') {
        result.reason = `reference SVG is ${referenceGeometry.roledescription ?? 'not a diagram'}, not a flowchart`;
        console.log(`! ${file}  ${result.reason}`);
        continue;
      }
      if (referenceGeometry.foreignObjects > 0) {
        foreignObjectFixtures++;
        result.warnings.push(
          `reference draws ${referenceGeometry.foreignObjects} foreignObject element(s): the forced htmlLabels did not apply`
        );
      }

      const excluded = exclusionReason(fixtureSource);
      let ferrigura: { svg: string; geometry: SvgGeometry } | null = null;
      if (excluded !== null) {
        result.class = 'excluded';
        result.reason = excluded;
        console.log(`– ${file}  excluded: ${excluded}`);
      } else {
        const rendered = renderFerrigura(text, id);
        if ('error' in rendered) {
          result.class = 'not rendered';
          result.reason = rendered.error;
          console.log(`· ${file}  not rendered: ${rendered.error}`);
        } else {
          const geometry = await measure(page, rendered.svg);
          const { worst, problems } = compareGeometry(referenceGeometry, geometry);
          ferrigura = { svg: rendered.svg, geometry };
          result.class = problems.length === 0 ? 'within' : 'outside';
          result.worstPercent = worst === null ? null : worst * 100;
          result.problems = problems;
          const mark = result.class === 'within' ? '✓' : '✗';
          const worstText =
            worst === null
              ? 'no numeric comparison'
              : `worst ${(worst * 100).toFixed(1)}% of tolerance`;
          console.log(`${mark} ${file}  ${worstText}`);
          for (const problem of problems.slice(0, 5)) console.log(`    ${problem}`);
          if (options.imageDir) {
            await page.evaluate(
              ([left, right]) =>
                (window as unknown as { sideBySide: (a: string, b: string) => void }).sideBySide(
                  left,
                  right
                ),
              [reference.svg, rendered.svg] as const
            );
            await mkdir(options.imageDir, { recursive: true });
            await page
              .locator('#stage > div')
              .screenshot({ path: join(options.imageDir, `flowchart-${slug}.png`) });
          }
        }
      }

      if (dumpSlugs.has(slug)) {
        await writeJson(join(workDir, `dump-${slug}.json`), {
          fixture: file,
          id,
          text,
          reference: { svg: reference.svg, geometry: referenceGeometry, elk: reference.elk },
          ferrigura: ferrigura && { svg: ferrigura.svg, geometry: ferrigura.geometry },
        });
      }
      if (options.capture) {
        const directory = join(workDir, 'flowchart', slug);
        await mkdir(directory, { recursive: true });
        await writeFile(join(directory, 'reference.svg'), reference.svg);
        await writeJson(join(directory, 'reference-geometry.json'), referenceGeometry);
        await writeJson(
          join(directory, 'elk-input.json'),
          reference.elk.map((call) => call.input)
        );
        await writeJson(
          join(directory, 'elk-output.json'),
          reference.elk.map((call) => call.output)
        );
      }
      // ELK: the example's input for the same text, against the reference's first call.
      if (elkProblem === null && result.class !== 'excluded' && reference.elk.length > 0) {
        const example = runElkExample(text, id);
        if ('error' in example) {
          result.elk.status = 'not available';
          result.elk.problems = [example.error];
        } else {
          const comparison = compareElkInput(reference.elk[0].input as ElkElement, example.input);
          result.elk.status = comparison.status;
          result.elk.problems = comparison.problems;
        }
      }
    }
  } finally {
    await browser.close();
    server.close();
  }

  const count = (name: FixtureClass) => results.filter((result) => result.class === name).length;
  const excludedBy = new Map<string, number>();
  const notRenderedBy = new Map<string, number>();
  for (const result of results) {
    if (result.class === 'excluded' && result.reason) {
      const key = result.reason.replace(/:.*$/, '');
      excludedBy.set(key, (excludedBy.get(key) ?? 0) + 1);
    }
    if (result.class === 'not rendered' && result.reason) {
      notRenderedBy.set(result.reason, (notRenderedBy.get(result.reason) ?? 0) + 1);
    }
  }
  const summary = {
    fixtures: results.length,
    within: count('within'),
    outside: count('outside'),
    notRendered: count('not rendered'),
    excluded: count('excluded'),
    referenceFailed: count('reference failed'),
    excludedBy: Object.fromEntries(excludedBy),
    notRenderedBy: Object.fromEntries(notRenderedBy),
    foreignObjectFixtures,
  };
  const elkCompared = results.filter(
    (result) => result.elk.status === 'match' || result.elk.status === 'differ'
  );
  const elkMatch = results.filter((result) => result.elk.status === 'match').length;

  await writeJson(join(workDir, 'flowchart-geometry.json'), {
    tolerances: { pixels: PIXEL_TOLERANCE },
    elkExample: elkProblem === null ? 'available' : `not available: ${elkProblem}`,
    summary,
    results,
  });

  console.log(`\nflowchart geometry: ${results.length} fixtures`);
  console.log(`  within tolerance:    ${summary.within}`);
  console.log(`  outside tolerance:   ${summary.outside}`);
  console.log(
    `  not rendered:        ${summary.notRendered} (${[...notRenderedBy].map(([reason, n]) => `${n} x ${reason}`).join('; ') || 'none'})`
  );
  console.log(
    `  excluded:            ${summary.excluded} (${[...excludedBy].map(([reason, n]) => `${n} x ${reason}`).join('; ') || 'none'})`
  );
  console.log(`  reference failed:    ${summary.referenceFailed}`);
  console.log(`  foreignObject in reference: ${foreignObjectFixtures} fixtures (should be 0)`);
  if (elkProblem === null) {
    console.log(
      `  ELK input compared:  ${elkCompared.length} (${elkMatch} match, ${elkCompared.length - elkMatch} differ)`
    );
  }
  if (options.capture) console.log(`  captured under ${join(workDir, 'flowchart')}`);
  if (summary.outside > 0) process.exit(1);
}

try {
  await main();
} catch (error) {
  // Usage errors (an unknown stage, a listed fixture that is missing) print one line, not a stack.
  console.error((error as Error).message);
  process.exit(2);
}
