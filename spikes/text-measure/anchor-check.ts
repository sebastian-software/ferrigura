/**
 * Where do WebKit and Chromium draw a kerned line, depending on how it is placed?
 *
 * WebKit lays out SVG text from per-character widths without kerning, but paints each text
 * fragment shaped, with kerning. Whatever it derives from the layout width (text-anchor offsets,
 * the start of a following tspan) is therefore off by the kerning of the line. This script draws
 * the kerning test line in five placements and compares the ink in both engines.
 *
 * Usage: `tsx anchor-check.ts`
 */
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

import { chromium, webkit } from 'playwright';

const { PNG } = await import(
  pathToFileURL('../../node_modules/.pnpm/pngjs@7.0.0/node_modules/pngjs/lib/png.js').href
);
const log = (line: string) => process.stdout.write(`${line}\n`);

const font = readFileSync('fonts/LexicaUltralegible-Regular.ttf').toString('base64');
const style = `<style>@font-face{font-family:lab;src:url(data:font/ttf;base64,${font})}</style>`;
const first = 'AVATAR Type';
const second = ' WAVE Yo Te LT';
const text = (attributes: string, body: string) =>
  `<text y="60" font-family="lab" font-size="48" style="white-space:pre" ${attributes}>${body}</text>`;
const cases: [string, (firstWidth: number) => string][] = [
  ['text-anchor start, x', () => text('x="10"', first + second)],
  ['text-anchor middle', () => text('x="450" text-anchor="middle"', first + second)],
  ['text-anchor end', () => text('x="890" text-anchor="end"', first + second)],
  [
    'second tspan without x',
    () => text('x="10"', `<tspan>${first}</tspan><tspan>${second}</tspan>`),
  ],
  [
    'second tspan with its x',
    (w) => text('x="10"', `<tspan>${first}</tspan><tspan x="${10 + w}">${second}</tspan>`),
  ],
];

/** Leftmost and rightmost column with dark pixels. */
const ink = (png: Buffer): [number, number] => {
  const image = PNG.sync.read(png);
  let left = Infinity;
  let right = -1;
  for (let y = 0; y < image.height; y++) {
    for (let x = 0; x < image.width; x++) {
      if (image.data[(y * image.width + x) * 4] < 128) {
        left = Math.min(left, x);
        right = Math.max(right, x);
      }
    }
  }
  return [left, right];
};

const results: Record<string, [number, number][]> = {};
for (const [name, engine] of [
  ['chromium', chromium],
  ['webkit', webkit],
] as const) {
  const browser = await engine.launch();
  const page = await browser.newPage({ viewport: { width: 900, height: 90 } });
  await page.setContent(`<!doctype html>${style}<body>`);
  await page.evaluate(() => document.fonts.load('48px lab'));
  // Canvas measures kerned text in both engines, as the Rust measurement does.
  const firstWidth = await page.evaluate((t) => {
    const context = document.createElement('canvas').getContext('2d')!;
    context.font = '48px lab';
    return context.measureText(t).width;
  }, first);
  results[name] = [];
  for (const [, svg] of cases) {
    await page.setContent(
      `<!doctype html>${style}<body style="margin:0;background:#fff"><svg xmlns="http://www.w3.org/2000/svg" width="900" height="90" style="display:block">${svg(firstWidth)}</svg>`
    );
    await page.evaluate(async () => {
      await document.fonts.load('48px lab');
      await document.fonts.ready;
    });
    await page.waitForTimeout(100);
    results[name].push(ink(await page.screenshot()));
  }
  await browser.close();
}

cases.forEach(([label], i) => {
  const [cl, cr] = results.chromium[i];
  const [wl, wr] = results.webkit[i];
  log(
    `${label.padEnd(26)} Chromium ink ${cl}..${cr}  WebKit ink ${wl}..${wr}  WebKit shift ${wl - cl} / ${wr - cr} px`
  );
});
