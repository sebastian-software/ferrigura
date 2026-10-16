/**
 * Regenerates `crates/ferrigura-text/tests/fixtures/chromium.json`: the widths, styled runs,
 * wrapped lines and box metrics that Chromium gives for the requests in that file.
 *
 * The requests stay as the fixture has them (text, style, size, wrap width), so a run measures the
 * same inputs again and writes the values it gets. `--check` compares them with the committed
 * values and exits non-zero instead of writing.
 *
 * Widths are the drawn extent of a `<text>` element: from the start of its first character to the
 * end of its last one. Box metrics are read from `getBBox` of a `<text>` at a known baseline: the box
 * top gives the ascent and the box height gives the descent. The canvas font bounding box must give
 * the same whole pixels, and the run fails if it does not. Chromium sets SVG text at the size floored
 * to 1/100 px, so the fixture's fractional sizes check that as well. Wrapped lines run Mermaid's
 * `splitLineToFitWidth` (`packages/mermaid/src/rendering-util/splitText.ts`) in the page, with the
 * width of a candidate line measured in SVG. `getComputedTextLength` and `getBBox` are used only
 * here, in Chromium: WebKit reports widths without kerning through them, and its box heights differ
 * by up to 0.38 px (`docs-port/spikes/text-measurement.md`).
 *
 * Usage: `pnpm exec tsx tools/port/text/chromium-fixture.ts [--check]`
 */
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { chromium } from 'playwright';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const fixturePath = join(root, 'crates/ferrigura-text/tests/fixtures/chromium.json');
const fontDir = join(root, 'crates/ferrigura-text/fonts');

/** The font file of each style, as the fixture names them. */
const FACES: Record<string, string> = {
  'sans-regular': 'LexicaUltralegible-Regular.ttf',
  'sans-italic': 'LexicaUltralegible-Italic.ttf',
  'sans-bold': 'LexicaUltralegible-Bold.ttf',
  'sans-bolditalic': 'LexicaUltralegible-BoldItalic.ttf',
  'mono-regular': 'AtkinsonHyperlegibleMono-Regular.ttf',
  'mono-italic': 'AtkinsonHyperlegibleMono-Italic.ttf',
  'mono-bold': 'AtkinsonHyperlegibleMono-Bold.ttf',
  'mono-bolditalic': 'AtkinsonHyperlegibleMono-BoldItalic.ttf',
};

/** The tolerance of widths, in pixels: Chromium lays text out in units of 1/64 px. */
const WIDTH_TOLERANCE = 0.02;

interface Fixture {
  widths: { text: string; style: string; size: number; width: number }[];
  runs: { runs: { text: string; style: string }[]; size: number; width: number }[];
  wraps: {
    text: string;
    style: string;
    size: number;
    wrapWidth: number;
    lines: string[];
  }[];
  boxes: { style: string; size: number; ascent: number; descent: number }[];
}

/** The page's functions, declared for TypeScript; `pageScript` defines them in the page. */
declare function drawnExtent(
  parts: { text: string; style: string | null }[],
  size: number,
  style: string | null
): number;
declare function wrapText(text: string, size: number, style: string, wrapWidth: number): string[];
declare function boxMetrics(style: string, size: number): { ascent: number; descent: number };

/** The page script: measurement and Mermaid's line splitting, as plain JavaScript. */
const pageScript = `
const svgNS = 'http://www.w3.org/2000/svg';

function svgText(parts, size, style) {
  const svg = document.createElementNS(svgNS, 'svg');
  svg.setAttribute('width', '4000');
  svg.setAttribute('height', '200');
  document.body.appendChild(svg);
  const text = document.createElementNS(svgNS, 'text');
  text.setAttribute('x', '0');
  text.setAttribute('y', '100');
  text.setAttribute('font-size', String(size));
  text.style.whiteSpace = 'pre';
  if (style) text.style.fontFamily = 'ft-' + style;
  for (const part of parts) {
    const tspan = document.createElementNS(svgNS, 'tspan');
    tspan.textContent = part.text;
    if (part.style) tspan.style.fontFamily = 'ft-' + part.style;
    text.appendChild(tspan);
  }
  svg.appendChild(text);
  return { svg, text };
}

function drawnExtent(parts, size, style) {
  const { svg, text } = svgText(parts, size, style);
  const count = text.getNumberOfChars();
  const width = text.getEndPositionOfChar(count - 1).x - text.getStartPositionOfChar(0).x;
  svg.remove();
  return width;
}

/** The width of a line of words, each word after the first in its own tspan with a leading space. */
function lineWidth(words, size, style) {
  const parts = words.map((word, index) => ({ text: (index === 0 ? '' : ' ') + word.content, style: null }));
  const { svg, text } = svgText(parts, size, style);
  const width = text.getComputedTextLength();
  svg.remove();
  return width;
}

const segmentChars = (text) => [...new Intl.Segmenter(undefined, { granularity: 'grapheme' }).segment(text)].map((s) => s.segment);

function splitWordToFitWidth(checkFit, word) {
  const characters = segmentChars(word.content);
  const usedChars = [];
  let remaining = characters.slice();
  while (true) {
    if (remaining.length === 0) {
      return [{ content: usedChars.join(''), type: word.type }, { content: '', type: word.type }];
    }
    const nextChar = remaining[0];
    const newWord = [...usedChars, nextChar];
    if (checkFit([{ content: newWord.join(''), type: word.type }])) {
      usedChars.push(nextChar);
      remaining = remaining.slice(1);
      continue;
    }
    if (usedChars.length === 0 && nextChar) {
      usedChars.push(nextChar);
      remaining = remaining.slice(1);
    }
    return [{ content: usedChars.join(''), type: word.type }, { content: remaining.join(''), type: word.type }];
  }
}

function splitLineToFitWidth(line, checkFit) {
  const lines = [];
  const words = line.slice();
  let newLine = [];
  while (true) {
    if (words.length === 0) {
      if (newLine.length > 0) lines.push(newLine);
      return lines;
    }
    let joiner = '';
    if (words[0].content === ' ') {
      joiner = ' ';
      words.shift();
    }
    const nextWord = words.shift() ?? { content: ' ', type: 'normal' };
    const lineWithNextWord = [...newLine];
    if (joiner !== '') lineWithNextWord.push({ content: joiner, type: 'normal' });
    lineWithNextWord.push(nextWord);
    if (checkFit(lineWithNextWord)) {
      newLine = lineWithNextWord;
      continue;
    }
    if (newLine.length > 0) {
      lines.push(newLine);
      words.unshift(nextWord);
    } else if (nextWord.content) {
      const [first, rest] = splitWordToFitWidth(checkFit, nextWord);
      lines.push([first]);
      if (rest.content) words.unshift(rest);
    }
    newLine = [];
  }
}

/**
 * The words of a label are the non-empty pieces of its text split at single spaces, as
 * \`markdownToLines\` makes them (handle-markdown-text.ts); the lines are joined with one space.
 */
function wrapText(text, size, style, wrapWidth) {
  const words = text.split(' ').filter((word) => word !== '').map((word) => ({ content: word, type: 'normal' }));
  const checkFit = (line) => lineWidth(line, size, style) <= wrapWidth;
  return splitLineToFitWidth(words, checkFit).map((line) => line.map((word) => word.content).join(' '));
}

/** The ascent and descent of a face at a size, from the box of a \`<text>\` whose baseline is at y = 100. */
function boxMetrics(style, size) {
  const baseline = 100;
  const { svg, text } = svgText([{ text: 'Hxg', style: null }], size, style);
  const box = text.getBBox();
  svg.remove();
  const ascent = baseline - box.y;
  const descent = box.height - ascent;
  const context = document.createElement('canvas').getContext('2d');
  context.font = size + 'px ft-' + style;
  const metrics = context.measureText('Hxg');
  if (metrics.fontBoundingBoxAscent !== ascent || metrics.fontBoundingBoxDescent !== descent) {
    throw new Error('canvas and SVG boxes differ for ' + style + ' at ' + size + 'px');
  }
  return { ascent, descent };
}
`;

async function main(): Promise<void> {
  const check = process.argv.includes('--check');
  const committed = JSON.parse(await readFile(fixturePath, 'utf8')) as Fixture;

  const faces = await Promise.all(
    Object.entries(FACES).map(async ([style, file]) => {
      const data = (await readFile(join(fontDir, file))).toString('base64');
      return `@font-face { font-family: "ft-${style}"; src: url(data:font/ttf;base64,${data}); }`;
    })
  );

  const browser = await chromium.launch({
    // Without hinting, Chromium on Linux keeps fractional glyph advances, as it does on macOS;
    // with the default, it rounds them to whole pixels and text measures differently.
    args: ['--font-render-hinting=none'],
  });
  try {
    const page = await browser.newPage();
    await page.setContent(
      `<!doctype html><html><head><style>${faces.join('\n')}</style></head><body></body></html>`
    );
    await page.evaluate(async (styles) => {
      for (const style of styles) {
        await document.fonts.load(`16px "ft-${style}"`);
      }
      await document.fonts.ready;
    }, Object.keys(FACES));
    // A script tag keeps the page's functions as globals for the evaluations below.
    await page.addScriptTag({ content: pageScript });

    const measured: Fixture = await page.evaluate((fixture: Fixture) => {
      const widths = fixture.widths.map((request) => ({
        text: request.text,
        style: request.style,
        size: request.size,
        width: drawnExtent([{ text: request.text, style: null }], request.size, request.style),
      }));
      const runs = fixture.runs.map((request) => ({
        runs: request.runs,
        size: request.size,
        width: drawnExtent(request.runs, request.size, null),
      }));
      const wraps = fixture.wraps.map((request) => ({
        text: request.text,
        style: request.style,
        size: request.size,
        wrapWidth: request.wrapWidth,
        lines: wrapText(request.text, request.size, request.style, request.wrapWidth),
      }));
      const boxes = fixture.boxes.map((request) => ({
        style: request.style,
        size: request.size,
        ...boxMetrics(request.style, request.size),
      }));
      return { widths, runs, wraps, boxes };
    }, committed);

    if (check) {
      const problems: string[] = [];
      measured.widths.forEach((entry, index) => {
        const was = committed.widths[index];
        if (Math.abs(entry.width - was.width) > WIDTH_TOLERANCE) {
          problems.push(
            `width ${entry.text} ${entry.style} ${entry.size}: ${entry.width} vs ${was.width}`
          );
        }
      });
      measured.runs.forEach((entry, index) => {
        if (Math.abs(entry.width - committed.runs[index].width) > WIDTH_TOLERANCE) {
          problems.push(`run line ${index}: ${entry.width} vs ${committed.runs[index].width}`);
        }
      });
      measured.wraps.forEach((entry, index) => {
        if (JSON.stringify(entry.lines) !== JSON.stringify(committed.wraps[index].lines)) {
          problems.push(`wrap ${entry.text} at ${entry.wrapWidth}: ${JSON.stringify(entry.lines)}`);
        }
      });
      measured.boxes.forEach((entry, index) => {
        const was = committed.boxes[index];
        if (entry.ascent !== was.ascent || entry.descent !== was.descent) {
          problems.push(`box ${entry.style} ${entry.size}: ${entry.ascent}/${entry.descent}`);
        }
      });
      if (problems.length > 0) {
        console.error(problems.join('\n'));
        console.error(`${problems.length} differences from ${fixturePath}`);
        process.exit(1);
      }
      console.log(
        `chromium fixture: ${measured.widths.length} widths, ${measured.runs.length} runs, ` +
          `${measured.wraps.length} wraps and ${measured.boxes.length} boxes agree with the committed values`
      );
      return;
    }

    await writeFile(fixturePath, `${JSON.stringify(measured, null, 2)}\n`);
    console.log(`wrote ${fixturePath}`);
  } finally {
    await browser.close();
  }
}

await main();
