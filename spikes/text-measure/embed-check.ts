/**
 * Check the SVGs written by `cargo run --bin embed` in Chromium:
 *
 * 1. `all.svg`, `all-plain.svg` and `all-length.svg` opened as documents: every embedded face
 *    loads, and every label is laid out at the width and text box height Rust measured
 *    (`data-width`, `data-height`), by `getComputedTextLength` and by the width of `getBBox`.
 * 2. The same SVG shown through `<img>`: pixel-identical to the document, so the embedded fonts
 *    also load where scripts and external resources are off.
 * 3. Two diagrams inlined in one HTML page, once with unique font family names and once with a
 *    shared one: whether one diagram's subset breaks the other's labels.
 *
 * Usage: `tsx embed-check.ts <out-dir> [chromium|firefox|webkit]`
 */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { chromium, firefox, type Page, webkit } from 'playwright';

const dir = resolve(process.argv[2]);
const url = (name: string) => pathToFileURL(resolve(dir, name)).href;

const engine = { chromium, firefox, webkit }[process.argv[3] ?? 'chromium']!;
const browser = await engine.launch();
const page = await browser.newPage({ viewport: { width: 1200, height: 900 } });
// tsx/esbuild wraps named functions in `__name(...)`, which the page does not define.
const shim = 'window.__name = (f) => f';
const log = (line: string) => process.stdout.write(`${line}\n`);

interface Check {
  texts: number;
  maxWidthDiff: number;
  maxBoxDiff: number;
  maxHeightDiff: number;
  over: number;
  overBox: number;
}

/** Compares every `<text data-width>` under `root` with what the browser lays out. */
async function measure(target: Page, root: string): Promise<Check> {
  await target.evaluate(shim);
  return target.evaluate(async (selector) => {
    await Promise.all([...document.fonts].map((face) => face.load()));
    await document.fonts.ready;
    const texts = [...document.querySelectorAll<SVGTextElement>(`${selector} text[data-width]`)];
    let maxWidthDiff = 0;
    let maxBoxDiff = 0;
    let maxHeightDiff = 0;
    let over = 0;
    let overBox = 0;
    for (const text of texts) {
      const box = text.getBBox();
      const widthDiff = Math.abs(text.getComputedTextLength() - Number(text.dataset.width));
      const boxDiff = Math.abs(box.width - Number(text.dataset.width));
      const heightDiff = Math.abs(box.height - Number(text.dataset.height));
      maxWidthDiff = Math.max(maxWidthDiff, widthDiff);
      maxBoxDiff = Math.max(maxBoxDiff, boxDiff);
      maxHeightDiff = Math.max(maxHeightDiff, heightDiff);
      over += widthDiff > 0.02 ? 1 : 0;
      overBox += boxDiff > 0.02 ? 1 : 0;
    }
    return { texts: texts.length, maxWidthDiff, maxBoxDiff, maxHeightDiff, over, overBox };
  }, root);
}

const report = (label: string, c: Check) =>
  log(
    `${label.padEnd(34)} ${String(c.texts).padStart(5)} labels  length max ${c.maxWidthDiff.toFixed(4)} px (${c.over} over 0.02)  box max ${c.maxBoxDiff.toFixed(4)} px (${c.overBox} over 0.02)  height max ${c.maxHeightDiff.toFixed(4)} px`
  );

// 1. As a document.
for (const name of ['all.svg', 'all-plain.svg', 'all-length.svg']) {
  await page.goto(url(name));
  report(name, await measure(page, 'svg'));
}
const faces = await page.evaluate(() =>
  [...document.fonts].map((f) => `${f.family} ${f.weight} ${f.style}: ${f.status}`)
);
log(`  faces: ${faces.join(', ')}`);

// 2. Through <img>, compared pixel by pixel with the document, and with an <img> of the same SVG
//    without its fonts as a control. The image is a data URL: an about:blank page may not load
//    file URLs. Clipped, since the SVGs are tall.
const dataUrl = (svg: string) => `data:image/svg+xml;base64,${Buffer.from(svg).toString('base64')}`;
const asImage = async (svg: string, clip: { width: number; height: number }) => {
  await page.goto('about:blank');
  await page.setContent(`<!doctype html><body style="margin:0"><img src="${dataUrl(svg)}"></body>`);
  await page.waitForFunction(() =>
    [...document.images].every((i) => i.complete && i.naturalWidth > 0)
  );
  await page.waitForTimeout(300);
  return page.screenshot({ clip: { x: 0, y: 0, ...clip } });
};
for (const name of ['half-b.svg', 'half-a.svg']) {
  const svg = readFileSync(resolve(dir, name), 'utf8');
  const size = /width="([\d.]+)" height="([\d.]+)"/.exec(svg)!;
  const clip = { width: Number(size[1]), height: Math.min(Number(size[2]), 6000) };
  await page.setViewportSize({ width: Math.ceil(clip.width), height: Math.ceil(clip.height) });
  await page.goto(url(name));
  await measure(page, 'svg');
  const asDocument = await page.screenshot({ clip: { x: 0, y: 0, ...clip } });
  const withFonts = await asImage(svg, clip);
  const withoutFonts = await asImage(svg.replace(/@font-face{[^}]*}/g, ''), clip);
  log(
    `${(name + ' via <img>').padEnd(34)} identical to document: ${asDocument.equals(withFonts)}; control without fonts identical: ${asDocument.equals(withoutFonts)}`
  );
}

// 3. Two diagrams in one page.
await page.setViewportSize({ width: 1200, height: 900 });
await page.goto('about:blank');
for (const [label, a, b] of [
  ['unique family names', 'half-a.svg', 'half-b.svg'],
  ['shared family name', 'shared-a.svg', 'shared-b.svg'],
]) {
  const html = `<!doctype html><body>
    <div id="a">${readFileSync(resolve(dir, a), 'utf8')}</div>
    <div id="b">${readFileSync(resolve(dir, b), 'utf8')}</div></body>`;
  await page.setContent(html);
  report(`${label}, diagram A`, await measure(page, '#a'));
  report(`${label}, diagram B`, await measure(page, '#b'));
}

await browser.close();
