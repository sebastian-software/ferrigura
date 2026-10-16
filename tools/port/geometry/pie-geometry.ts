/**
 * Geometry comparison of pie diagrams: the JavaScript reference and ferrigura, for every pie
 * fixture under `e2e/diagrams/pie`.
 *
 * Both sides get the text patched by `patchText` (`common.ts`): the forced directive, with the
 * theme redux-color for a fixture that names none. The reference is Mermaid's built bundle
 * (`packages/mermaid/dist`), run in Chromium with Lexica Ultralegible loaded as a web font, so that
 * it measures the same glyphs ferrigura does. ferrigura renders the same text with its `render`
 * example. Both SVGs are then measured in the same Chromium page, at scale 1 and in the coordinates
 * of their viewBox, and the geometry is compared: the viewBox, the outer circle, the start and end
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
 * Usage: `pnpm port:geometry-pie [--images <dir>]`
 */
import { readFile } from 'node:fs/promises';
import { basename, join } from 'node:path';

import {
  buildRenderExample,
  flagValue,
  measureSvg,
  mmdFiles,
  patchText,
  PIXEL_TOLERANCE,
  renderFerrigura,
  renderReference,
  root,
  slugOf,
  withPage,
  workDir,
  writeJson,
  writeSideBySide,
} from './common.js';

const fixtureDir = join(root, 'e2e/diagrams/pie');

const ANGLE_TOLERANCE = 1e-5;

/**
 * The page-side measurement of one pie SVG, as source text for the page (`pageHtml` in `common.ts`).
 * `measure` gets the SVG on the stage at scale 1 and returns its slices, label boxes, title and
 * legend; `geometry` in the page adds the attributes of the root.
 */
const MEASURE = `
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
const measure = (svg) => {
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
};`;

interface PieResult {
  fixture: string;
  ok: boolean;
  worst: number | null;
  problems: string[];
}

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

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  const imageDir = flagValue(argv, '--images');
  buildRenderExample();

  const files = await mmdFiles(fixtureDir);
  const results: PieResult[] = [];
  await withPage(MEASURE, async (page) => {
    for (const file of files) {
      const name = basename(file, '.mmd');
      const id = `ref-${slugOf(file)}`;
      const source = await readFile(join(fixtureDir, file), 'utf8');
      const text = patchText(source);
      const reference = await renderReference(page, id, text);
      if (!reference.ok || reference.svg === null) {
        results.push({ fixture: file, ok: false, worst: null, problems: [reference.error ?? ''] });
        console.log(`✗ ${file}  reference failed: ${reference.error}`);
        continue;
      }
      const rendered = renderFerrigura(text, id);
      if ('error' in rendered) {
        results.push({ fixture: file, ok: false, worst: null, problems: [rendered.error] });
        console.log(`✗ ${file}  not rendered: ${rendered.error}`);
        continue;
      }
      const jsGeometry = await measureSvg<Geometry>(page, reference.svg);
      const rsGeometry = await measureSvg<Geometry>(page, rendered.svg);
      if (process.env.DUMP === name) {
        await writeJson(join(workDir, `dump-${name}.json`), {
          js: jsGeometry,
          rs: rsGeometry,
          jsSvg: reference.svg,
          rsSvg: rendered.svg,
        });
      }
      const { worst, problems } = compare(jsGeometry, rsGeometry);
      results.push({ fixture: file, ok: problems.length === 0, worst, problems });
      console.log(
        `${problems.length === 0 ? '✓' : '✗'} ${file}  worst ${(worst * 100).toFixed(1)}% of tolerance`
      );
      for (const problem of problems.slice(0, 5)) console.log(`    ${problem}`);

      if (imageDir) {
        await writeSideBySide(page, reference.svg, rendered.svg, join(imageDir, `pie-${name}.png`));
      }
    }
  });

  const failed = results.filter((result) => !result.ok);
  await writeJson(join(workDir, 'pie-geometry.json'), {
    tolerances: { angle: ANGLE_TOLERANCE, pixels: PIXEL_TOLERANCE },
    results,
  });
  console.log(
    `\npie geometry: ${results.length} fixtures, ${results.length - failed.length} within tolerance, ${failed.length} outside`
  );
  if (failed.length > 0) process.exit(1);
}

await main();
