/**
 * Geometry comparison of pie diagrams: the JavaScript reference and ferrigura, for every pie
 * fixture under `e2e/diagrams/pie`.
 *
 * The reference is Mermaid's built bundle (`packages/mermaid/dist`), run in Chromium with
 * `htmlLabels: false`, the theme `redux-color` and Lexica Ultralegible loaded as a web font, so
 * that it measures the same glyphs ferrigura does. ferrigura renders the same text with its
 * `render` example. Both SVGs are then measured in the same Chromium page, in the coordinates of
 * their viewBox, and the geometry is compared: the viewBox, the outer circle, the start and end
 * angles of every slice, the box of every slice label, the title and the legend entries.
 *
 * Chromium is used for the measurement on purpose. WebKit reports kerned text widths differently
 * through `getBBox`, so this comparison is not a WebKit check.
 *
 * Tolerances: angles 1e-5 rad; positions and sizes 0.05 px. The reference writes its path
 * coordinates with three decimals (d3-path), so an endpoint is off by up to 0.0005 px, which is
 * 2.7e-6 rad on the outer radius of 185 px; 1e-5 rad is four times that.
 * Chromium lays text out in units of 1/64 px (0.0156 px), and a label is a few glyphs wide, so
 * a width can differ from the reference by a few hundredths of a pixel. 0.05 px is that
 * difference with room to spare, and far below anything a reader can see.
 *
 * Usage: `pnpm exec tsx tools/port/geometry/pie-geometry.ts [--images <dir>]`
 */
import { execFileSync } from 'node:child_process';
import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { extname, join, basename } from 'node:path';
import { fileURLToPath } from 'node:url';

import { chromium, type Page } from 'playwright';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const fixtureDir = join(root, 'e2e/diagrams/pie');
const distDir = join(root, 'packages/mermaid/dist');
const fontDir = join(root, 'crates/ferrigura-text/fonts');
const workDir = join(root, 'target/geometry');

const ANGLE_TOLERANCE = 1e-5;
const PIXEL_TOLERANCE = 0.05;

/** The site configuration of the reference: what ferrigura assumes, written as `initialize`. */
const referenceConfig = {
  htmlLabels: false,
  theme: 'redux-color',
  fontFamily: '"Lexica Ultralegible"',
  securityLevel: 'strict',
};

interface Box {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface Slice {
  start: number;
  end: number;
  label: Box;
}

interface Legend {
  transform: [number, number];
  rect: Box;
  text: Box;
}

interface Geometry {
  viewBox: [number, number, number, number];
  maxWidth: string | null;
  outerCircle: { cx: number; cy: number; r: number };
  slices: Slice[];
  title: Box | null;
  legend: Legend[];
}

/** The page: renders the reference and measures both kinds of SVG in the page's coordinates. */
const harness = `<!doctype html>
<html><head><meta charset="utf-8">
<style>
@font-face { font-family: "Lexica Ultralegible"; src: url(/fonts/LexicaUltralegible-Regular.ttf) format("truetype"); font-weight: 400; font-style: normal; }
body { margin: 0; background: #ffffff; }
#stage { width: 1000px; }
</style></head>
<body><div id="stage"></div>
<script type="module">
import mermaid from '/dist/mermaid.esm.mjs';
await document.fonts.load('16px "Lexica Ultralegible"');
window.renderReference = async (id, text, config) => {
  mermaid.initialize({ startOnLoad: false, ...config });
  const { svg } = await mermaid.render(id, text);
  return svg;
};
const boxIn = (svg, element) => {
  const box = element.getBBox();
  const matrix = element.getCTM();
  const corners = [
    [box.x, box.y], [box.x + box.width, box.y],
    [box.x, box.y + box.height], [box.x + box.width, box.y + box.height],
  ].map(([x, y]) => {
    const point = svg.createSVGPoint();
    point.x = x;
    point.y = y;
    const mapped = point.matrixTransform(matrix);
    return [mapped.x, mapped.y];
  });
  const xs = corners.map((c) => c[0]);
  const ys = corners.map((c) => c[1]);
  const x = Math.min(...xs);
  const y = Math.min(...ys);
  return { x, y, width: Math.max(...xs) - x, height: Math.max(...ys) - y };
};
const pointIn = (svg, element, x, y) => {
  const point = svg.createSVGPoint();
  point.x = x;
  point.y = y;
  const mapped = point.matrixTransform(element.getCTM());
  return [mapped.x, mapped.y];
};
window.geometry = async (svgText) => {
  const stage = document.getElementById("stage");
  stage.innerHTML = svgText;
  // The embedded faces load asynchronously; measuring before they load would use a fallback font.
  for (const [, family] of svgText.matchAll(/font-family: ?"([^"]+)"/g)) {
    await document.fonts.load('16px "' + family + '"');
  }
  await document.fonts.ready;
  const svg = stage.querySelector('svg');
  const viewBox = svg.getAttribute('viewBox').trim().split(/\\s+/).map(Number);
  const circle = svg.querySelector('.pieOuterCircle');
  const centre = pointIn(svg, circle, Number(circle.getAttribute('cx')), Number(circle.getAttribute('cy')));
  const angleOf = (point) => Math.atan2(point[1] - centre[1], point[0] - centre[0]);
  const slices = [...svg.querySelectorAll('path.pieCircle')].map((path) => {
    const d = path.getAttribute('d');
    const m = d.match(/^M([-0-9.e]+),([-0-9.e]+)A[^,]+,[^,]+,[^,]+,[^,]+,[^,]+,([-0-9.e]+),([-0-9.e]+)/);
    const start = pointIn(svg, path, Number(m[1]), Number(m[2]));
    const end = pointIn(svg, path, Number(m[3]), Number(m[4]));
    return { start: angleOf(start), end: angleOf(end), start_point: start, end_point: end };
  });
  const labels = [...svg.querySelectorAll('text.slice')].map((text) => boxIn(svg, text));
  const title = svg.querySelector('text.pieTitleText');
  const legend = [...svg.querySelectorAll('g.legend')].map((group) => {
    const transform = group.getAttribute('transform').match(/translate\\(([-0-9.e]+),\\s*([-0-9.e]+)\\)/);
    return {
      transform: [Number(transform[1]), Number(transform[2])],
      rect: boxIn(svg, group.querySelector('rect')),
      text: boxIn(svg, group.querySelector('text')),
    };
  });
  const rawWidth = svg.getAttribute('width');
  const style = svg.getAttribute('style');
  return {
    viewBox,
    maxWidth: style && style.includes('max-width') ? style : null,
    width: rawWidth,
    height: svg.getAttribute('height'),
    outerCircle: { cx: centre[0], cy: centre[1], r: boxIn(svg, circle).width / 2 },
    slices: slices.map((slice, index) => ({ start: slice.start, end: slice.end, label: labels[index] })),
    title: title ? boxIn(svg, title) : null,
    legend,
  };
};
window.sideBySide = (left, right) => {
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

/** Every `.mmd` file under a directory, at any depth. */
async function mmdFiles(directory: string): Promise<string[]> {
  const found: string[] = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) found.push(...(await mmdFiles(path)));
    else if (extname(entry.name) === '.mmd') found.push(path);
  }
  return found;
}

/** A static server for the harness, the bundle and the bundled fonts. */
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
      const type =
        extname(file) === '.ttf'
          ? 'font/ttf'
          : extname(file) === '.css'
            ? 'text/css'
            : 'text/javascript';
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

/** Maximum absolute difference of two numbers. */
function diff(a: number, b: number): number {
  return Math.abs(a - b);
}

/** The largest difference between two geometries, and the first place it occurs. */
function compare(js: Geometry, rs: Geometry): { worst: number; where: string; problems: string[] } {
  const problems: string[] = [];
  let worst = 0;
  const check = (value: number, tolerance: number, where: string) => {
    worst = Math.max(worst, value / tolerance);
    if (value > tolerance) problems.push(`${where}: ${value.toFixed(6)}`);
  };
  js.viewBox.forEach((value, index) =>
    check(diff(value, rs.viewBox[index]), PIXEL_TOLERANCE, `viewBox[${index}]`)
  );
  check(diff(js.outerCircle.r, rs.outerCircle.r), PIXEL_TOLERANCE, 'outer radius');
  check(diff(js.outerCircle.cx, rs.outerCircle.cx), PIXEL_TOLERANCE, 'centre x');
  check(diff(js.outerCircle.cy, rs.outerCircle.cy), PIXEL_TOLERANCE, 'centre y');
  if (js.slices.length !== rs.slices.length) {
    problems.push(`slice count: ${js.slices.length} vs ${rs.slices.length}`);
  }
  js.slices.forEach((slice, index) => {
    const other = rs.slices[index];
    if (!other) return;
    check(diff(slice.start, other.start), ANGLE_TOLERANCE, `slice ${index} start angle`);
    check(diff(slice.end, other.end), ANGLE_TOLERANCE, `slice ${index} end angle`);
    for (const key of ['x', 'y', 'width', 'height'] as const) {
      check(
        diff(slice.label[key], other.label[key]),
        PIXEL_TOLERANCE,
        `slice ${index} label ${key}`
      );
    }
  });
  if ((js.title === null) !== (rs.title === null)) {
    problems.push('title present in one output only');
  } else if (js.title && rs.title) {
    for (const key of ['x', 'y', 'width', 'height'] as const) {
      check(diff(js.title[key], rs.title[key]), PIXEL_TOLERANCE, `title ${key}`);
    }
  }
  if (js.legend.length !== rs.legend.length) {
    problems.push(`legend count: ${js.legend.length} vs ${rs.legend.length}`);
  }
  js.legend.forEach((entry, index) => {
    const other = rs.legend[index];
    if (!other) return;
    check(diff(entry.transform[0], other.transform[0]), PIXEL_TOLERANCE, `legend ${index} x`);
    check(diff(entry.transform[1], other.transform[1]), PIXEL_TOLERANCE, `legend ${index} y`);
    for (const key of ['x', 'y', 'width', 'height'] as const) {
      check(diff(entry.text[key], other.text[key]), PIXEL_TOLERANCE, `legend ${index} text ${key}`);
    }
  });
  return { worst, where: problems[0] ?? '', problems };
}

/** The ferrigura render of a fixture, through the `render` example. */
function renderFerrigura(text: string): string {
  return execFileSync(join(root, 'target/debug/examples/render'), {
    input: text,
    cwd: root,
    maxBuffer: 16 * 1024 * 1024,
  }).toString('utf8');
}

async function main(): Promise<void> {
  const imageArgument = process.argv.indexOf('--images');
  const imageDir = imageArgument >= 0 ? process.argv[imageArgument + 1] : undefined;
  execFileSync('cargo', ['build', '-q', '-p', 'ferrigura', '--example', 'render'], {
    cwd: root,
    stdio: 'inherit',
  });
  await mkdir(workDir, { recursive: true });

  const files = (await mmdFiles(fixtureDir))
    .map((path) => path.slice(fixtureDir.length + 1))
    .sort();
  const { server, port } = await serve();
  const browser = await chromium.launch({
    // Without hinting, Chromium on Linux keeps fractional glyph advances, as it does on macOS;
    // with the default, it rounds them to whole pixels and text measures differently.
    args: ['--font-render-hinting=none'],
  });
  const results: { fixture: string; ok: boolean; worst: number; problems: string[] }[] = [];
  try {
    const page: Page = await browser.newPage({ viewport: { width: 1200, height: 900 } });
    page.on('pageerror', (error) => console.error('page error:', error.message));
    await page.goto(`http://127.0.0.1:${port}/`);
    await page.waitForFunction(
      () => (window as unknown as { ready?: boolean }).ready === true,
      null,
      { timeout: 60_000 }
    );

    for (const file of files) {
      const text = await readFile(join(fixtureDir, file), 'utf8');
      const referenceSvg = await page.evaluate(
        ([id, source, config]) =>
          (
            window as unknown as { renderReference: (...a: unknown[]) => Promise<string> }
          ).renderReference(id, source, config),
        [`ref-${basename(file, '.mmd')}`, text, referenceConfig] as const
      );
      const ferrigura = renderFerrigura(text);
      const jsGeometry = (await page.evaluate(
        (svg) => (window as unknown as { geometry: (s: string) => Promise<unknown> }).geometry(svg),
        referenceSvg
      )) as Geometry;
      const rsGeometry = (await page.evaluate(
        (svg) => (window as unknown as { geometry: (s: string) => Promise<unknown> }).geometry(svg),
        ferrigura
      )) as Geometry;
      if (process.env.DUMP === basename(file, '.mmd')) {
        const dumpPath = join(workDir, `dump-${basename(file, '.mmd')}.json`);
        await writeFile(
          dumpPath,
          JSON.stringify(
            { js: jsGeometry, rs: rsGeometry, jsSvg: referenceSvg, rsSvg: ferrigura },
            null,
            2
          )
        );
      }
      const { worst, problems } = compare(jsGeometry, rsGeometry);
      results.push({ fixture: file, ok: problems.length === 0, worst, problems });
      console.log(
        `${problems.length === 0 ? '✓' : '✗'} ${file}  worst ${(worst * 100).toFixed(1)}% of tolerance`
      );
      for (const problem of problems.slice(0, 5)) console.log(`    ${problem}`);

      if (imageDir) {
        await page.evaluate(
          ([left, right]) =>
            (window as unknown as { sideBySide: (a: string, b: string) => void }).sideBySide(
              left,
              right
            ),
          [referenceSvg, ferrigura] as const
        );
        await mkdir(imageDir, { recursive: true });
        await page
          .locator('#stage > div')
          .screenshot({ path: join(imageDir, `pie-${basename(file, '.mmd')}.png`) });
      }
    }
  } finally {
    await browser.close();
    server.close();
  }

  const failed = results.filter((result) => !result.ok);
  await writeFile(
    join(workDir, 'pie-geometry.json'),
    JSON.stringify(
      { tolerances: { angle: ANGLE_TOLERANCE, pixels: PIXEL_TOLERANCE }, results },
      null,
      2
    )
  );
  console.log(
    `\npie geometry: ${results.length} fixtures, ${results.length - failed.length} within tolerance, ${failed.length} outside`
  );
  if (failed.length > 0) process.exit(1);
}

await main();
