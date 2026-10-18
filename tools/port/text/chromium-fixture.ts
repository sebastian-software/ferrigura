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
 * The `extents` section is the box `getBBox` gives a line of SVG text: the union of the advance of
 * each run and the outline of each glyph, horizontally (`ferrigura_text::measure_line_box`). Each
 * request is a line of styled runs at a size and an anchor, and the entry records the box's `x` and
 * `width` and the line's `getComputedTextLength`, which is the advance the anchor shifts it by. The
 * requests are built below: every printable ASCII glyph and some other glyphs in the four styles
 * (glyphs with ink past their advance, the leading `j`, the italic `f`, the trailing `q`), words
 * that overhang at either end, and lines whose runs change style.
 *
 * The `fallback` section is the same box for lines that mix a face with the symbol face: the arrows
 * the text faces lack are set in Noto Sans Math, in a font-family list of the text's own face and
 * `ft-symbols` (`ferrigura_text::fallback_runs`). Its entries carry the ascent and descent of the
 * line too, which are the text face's: the arrows do not change the box. `fallbackWraps` are
 * Mermaid's wrapped lines of such strings.
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
  symbols: 'NotoSansMath-Regular-Arrows.ttf',
};

/** The tolerance of widths, in pixels: Chromium lays text out in units of 1/64 px. */
const WIDTH_TOLERANCE = 0.02;

/** The tolerance of the `extents` boxes, in pixels: float32 values read from Chromium. */
const EXTENT_TOLERANCE = 0.001;

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
  extents: ExtentEntry[];
  fallback: FallbackEntry[];
  fallbackWraps: WrapEntry[];
}

/** A wrapped line of a string whose characters fall back to the symbol face. */
type WrapEntry = {
  text: string;
  style: string;
  size: number;
  wrapWidth: number;
  lines: string[];
};

/** A line of styled runs whose characters fall back to the symbol face, with its box. */
interface FallbackEntry {
  parts: { text: string; style: string }[];
  size: number;
  anchor: 'start' | 'middle' | 'end';
  /** The box's left edge relative to the anchor point, as `getBBox().x`. */
  x: number;
  /** The box's width, as `getBBox().width`. */
  width: number;
  /** The advance of the line, as `getComputedTextLength`. */
  computedLength: number;
  /** The ascent and descent of the line's box, from its baseline: the text faces' own. */
  ascent: number;
  descent: number;
}

/** A line of styled runs at a size, anchored as `text-anchor` says, with the box Chromium gives it. */
interface ExtentEntry {
  parts: { text: string; style: string }[];
  size: number;
  anchor: 'start' | 'middle' | 'end';
  /** The box's left edge relative to the anchor point, as `getBBox().x`. */
  x: number;
  /** The box's width, as `getBBox().width`. */
  width: number;
  /** The advance of the line, as `getComputedTextLength`. */
  computedLength: number;
}

// cspell:ignore Åßſÿ
/** The glyphs of the Lexica faces: printable ASCII and some other glyphs, at 14 px in four styles. */
const GLYPHS: string[] = [
  ...Array.from({ length: 0x7e - 0x21 + 1 }, (_, index) => String.fromCharCode(0x21 + index)),
  ...'éÅßſÿ“’',
];

/** The words that overhang at either end, and plain words for comparison. */
const WORDS: string[] = [
  'j',
  'jun',
  'jul',
  'jq',
  'junk',
  'quit',
  'Jj gq',
  'fj',
  'ay',
  'may',
  'Sales Revenue',
  '1',
  'x.',
  '(j)',
  'Yj',
  'Fjord',
];

/** The lines whose runs change style: a leading `j` or a trailing italic `f` reach past the box. */
const MIXED: { parts: { text: string; style: string }[] }[] = [
  {
    parts: [
      { text: 'j', style: 'sans-regular' },
      { text: ' un', style: 'sans-italic' },
    ],
  },
  {
    parts: [
      { text: 'Hel', style: 'sans-regular' },
      { text: 'lo f', style: 'sans-italic' },
    ],
  },
  {
    parts: [
      { text: 'j', style: 'sans-italic' },
      { text: ' gq', style: 'sans-regular' },
    ],
  },
  {
    parts: [
      { text: 'x ', style: 'sans-regular' },
      { text: 'f', style: 'sans-italic' },
    ],
  },
  {
    parts: [
      { text: 'q', style: 'sans-bold' },
      { text: ' fox', style: 'sans-regular' },
    ],
  },
  {
    parts: [
      { text: 'An', style: 'sans-italic' },
      { text: 'jf', style: 'sans-bold' },
    ],
  },
  {
    parts: [
      { text: 'fj', style: 'sans-bolditalic' },
      { text: ' ', style: 'sans-regular' },
      { text: 'y', style: 'sans-italic' },
    ],
  },
  {
    parts: [
      { text: 'Jj', style: 'sans-regular' },
      { text: ' gq', style: 'sans-bolditalic' },
      { text: 'f', style: 'sans-bold' },
    ],
  },
  { parts: [{ text: 'f', style: 'sans-bolditalic' }] },
  { parts: [{ text: 'jun', style: 'sans-italic' }] },
  { parts: [{ text: 'j', style: 'sans-italic' }] },
];

const STYLES = ['sans-regular', 'sans-italic', 'sans-bold', 'sans-bolditalic'];

/** A request of the `extents` section: the line and its anchor, without the measured box. */
type RequestOf = Omit<ExtentEntry, 'x' | 'width' | 'computedLength'>;

/** The requests of the `extents` section, in the order the fixture keeps them. */
function extentRequests(): RequestOf[] {
  const requests: RequestOf[] = [];
  for (const glyph of GLYPHS) {
    for (const style of STYLES) {
      requests.push({ parts: [{ text: glyph, style }], size: 14, anchor: 'start' });
    }
  }
  for (const word of WORDS) {
    for (const style of STYLES) {
      for (const size of [10, 12, 14, 16, 18.5]) {
        requests.push({ parts: [{ text: word, style }], size, anchor: 'start' });
      }
      for (const size of [12, 14, 18.5]) {
        for (const anchor of ['middle', 'end'] as const) {
          requests.push({ parts: [{ text: word, style }], size, anchor });
        }
      }
    }
  }
  for (const line of MIXED) {
    for (const size of [12, 14, 16]) {
      for (const anchor of ['start', 'middle', 'end'] as const) {
        requests.push({ parts: line.parts, size, anchor });
      }
    }
  }
  return requests;
}

/**
 * The strings of the `fallback` section: arrows the text faces lack, alone and in words, in every
 * style. The fallback is the arrows block of the symbol face and nothing else.
 */
const FALLBACK_WORDS: string[] = [
  '→',
  '→→',
  'a→b',
  'Hello → world',
  'j→',
  '→j',
  'x ⇔ yj',
  'Fjord ⇒ Oslo',
  '(↑)',
  'ab↕',
  '↔ f',
  'q↺ gq',
];
const FALLBACK_STYLES: string[] = [
  ...STYLES,
  'mono-regular',
  'mono-italic',
  'mono-bold',
  'mono-bolditalic',
];

/** The lines of styled runs that mix faces with arrows; the arrows fall back in each run. */
const FALLBACK_MIXED: { parts: { text: string; style: string }[] }[] = [
  {
    parts: [
      { text: 'Ab ', style: 'sans-regular' },
      { text: '→ ', style: 'sans-bold' },
      { text: 'Cd', style: 'sans-italic' },
    ],
  },
  {
    parts: [
      { text: 'j', style: 'sans-italic' },
      { text: ' → j', style: 'sans-regular' },
    ],
  },
  {
    parts: [
      { text: 'x', style: 'sans-bold' },
      { text: '↔', style: 'sans-bolditalic' },
      { text: 'y', style: 'sans-regular' },
    ],
  },
  {
    parts: [
      { text: '↑', style: 'sans-regular' },
      { text: ' fj', style: 'sans-bold' },
    ],
  },
  {
    parts: [
      { text: 'Hel', style: 'mono-regular' },
      { text: 'lo→', style: 'mono-bold' },
    ],
  },
];

/** The wrapped strings of the `fallbackWraps` section, at three widths in two styles. */
const FALLBACK_WRAP_TEXTS: string[] = [
  'Hello → world again',
  'Fjord ⇒ Oslo to Bergen',
  '→→→→→→→→→→→→',
  'a→b c→d e→f',
];

/** A request of the `fallback` section, without the measured box. */
type FallbackRequest = Omit<FallbackEntry, 'x' | 'width' | 'computedLength' | 'ascent' | 'descent'>;
/** A request of the `fallbackWraps` section, without the wrapped lines. */
type WrapRequest = Omit<WrapEntry, 'lines'>;

/** The requests of the `fallback` section, in the order the fixture keeps them. */
function fallbackRequests(): FallbackRequest[] {
  const requests: FallbackRequest[] = [];
  for (const word of FALLBACK_WORDS) {
    for (const style of FALLBACK_STYLES) {
      for (const size of [12, 14, 18.5]) {
        for (const anchor of ['start', 'middle', 'end'] as const) {
          requests.push({ parts: [{ text: word, style }], size, anchor });
        }
      }
    }
  }
  for (const line of FALLBACK_MIXED) {
    for (const size of [12, 14, 16]) {
      for (const anchor of ['start', 'middle', 'end'] as const) {
        requests.push({ parts: line.parts, size, anchor });
      }
    }
  }
  return requests;
}

/** The requests of the `fallbackWraps` section: each text in two styles at three widths. */
function fallbackWrapRequests(): WrapRequest[] {
  const requests: WrapRequest[] = [];
  for (const text of FALLBACK_WRAP_TEXTS) {
    for (const style of ['sans-regular', 'sans-bold']) {
      for (const wrapWidth of [40, 80, 120]) {
        requests.push({ text, style, size: 14, wrapWidth });
      }
    }
  }
  return requests;
}

/** The page's functions, declared for TypeScript; `pageScript` defines them in the page. */
declare function drawnExtent(
  parts: { text: string; style: string | null }[],
  size: number,
  style: string | null
): number;
declare function wrapText(
  text: string,
  size: number,
  style: string,
  wrapWidth: number,
  fallback?: boolean
): string[];
declare function fallbackBox(
  parts: { text: string; style: string }[],
  size: number,
  anchor: string
): { x: number; width: number; computedLength: number; ascent: number; descent: number };
declare function boxMetrics(style: string, size: number): { ascent: number; descent: number };
declare function lineBox(
  parts: { text: string; style: string | null }[],
  size: number,
  anchor: string
): { x: number; width: number; computedLength: number };

/** The page script: measurement and Mermaid's line splitting, as plain JavaScript. */
const pageScript = `
const svgNS = 'http://www.w3.org/2000/svg';

/** The font-family list of a style: its own face, then the symbol face when the text falls back. */
const familyOf = (style, fallback) => (fallback ? '"ft-' + style + '", "ft-symbols"' : 'ft-' + style);

function svgText(parts, size, style, fallback = false) {
  const svg = document.createElementNS(svgNS, 'svg');
  svg.setAttribute('width', '4000');
  svg.setAttribute('height', '200');
  document.body.appendChild(svg);
  const text = document.createElementNS(svgNS, 'text');
  text.setAttribute('x', '0');
  text.setAttribute('y', '100');
  text.setAttribute('font-size', String(size));
  text.style.whiteSpace = 'pre';
  if (style) text.style.fontFamily = familyOf(style, fallback);
  for (const part of parts) {
    const tspan = document.createElementNS(svgNS, 'tspan');
    tspan.textContent = part.text;
    if (part.style) tspan.style.fontFamily = familyOf(part.style, fallback);
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

/** The box and the advance of a line of styled runs, as getBBox and getComputedTextLength give them. */
function lineBox(parts, size, anchor) {
  const { svg, text } = svgText(parts, size, null);
  text.setAttribute('text-anchor', anchor);
  const box = text.getBBox();
  const computedLength = text.getComputedTextLength();
  svg.remove();
  return { x: box.x, width: box.width, computedLength };
}

/** The width of a line of words, each word after the first in its own tspan with a leading space. */
function lineWidth(words, size, style, fallback = false) {
  const parts = words.map((word, index) => ({ text: (index === 0 ? '' : ' ') + word.content, style: null }));
  const { svg, text } = svgText(parts, size, style, fallback);
  const width = text.getComputedTextLength();
  svg.remove();
  return width;
}

/** The box and the advance of a line of runs whose faces fall back to the symbol face. */
function fallbackBox(parts, size, anchor) {
  const { svg, text } = svgText(parts, size, null, true);
  text.setAttribute('text-anchor', anchor);
  const box = text.getBBox();
  const computedLength = text.getComputedTextLength();
  svg.remove();
  const ascent = 100 - box.y;
  return { x: box.x, width: box.width, computedLength, ascent, descent: box.height - ascent };
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
function wrapText(text, size, style, wrapWidth, fallback = false) {
  const words = text.split(' ').filter((word) => word !== '').map((word) => ({ content: word, type: 'normal' }));
  const checkFit = (line) => lineWidth(line, size, style, fallback) <= wrapWidth;
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

/**
 * The fixture as JSON, with one line per `extents` and `fallback` entry: the entries are many and
 * each is short, and one line each keeps the file reviewable.
 */
function formatFixture(fixture: Fixture): string {
  const { widths, runs, wraps, boxes, extents, fallback, fallbackWraps } = fixture;
  const head = JSON.stringify({ widths, runs, wraps, boxes }, null, 2);
  const lines = (items: unknown[]) =>
    items.map((entry) => `    ${JSON.stringify(entry)}`).join(',\n');
  const wrapLines = JSON.stringify(fallbackWraps, null, 2).replace(/\n/g, '\n  ');
  return (
    head.replace(
      /\n}$/,
      `,\n  "extents": [\n${lines(extents)}\n  ],\n  "fallback": [\n${lines(fallback)}\n  ],\n  "fallbackWraps": ${wrapLines}\n}`
    ) + '\n'
  );
}

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

    const requests = extentRequests();
    const fallbackLines = fallbackRequests();
    const wrapRequests = fallbackWrapRequests();
    const measured: Fixture = await page.evaluate(
      ([fixture, requests, fallbackLines, fallbackWrapRequests]: [
        Fixture,
        RequestOf[],
        FallbackRequest[],
        WrapRequest[],
      ]) => {
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
        const extents = requests.map((request) => ({
          ...request,
          ...lineBox(request.parts, request.size, request.anchor),
        }));
        const fallback = fallbackLines.map((request) => ({
          ...request,
          ...fallbackBox(request.parts, request.size, request.anchor),
        }));
        const fallbackWraps = fallbackWrapRequests.map((request) => ({
          ...request,
          lines: wrapText(request.text, request.size, request.style, request.wrapWidth, true),
        }));
        return { widths, runs, wraps, boxes, extents, fallback, fallbackWraps };
      },
      [committed, requests, fallbackLines, wrapRequests]
    );

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
      measured.extents.forEach((entry, index) => {
        const was = committed.extents?.[index];
        if (was === undefined || JSON.stringify(entry.parts) !== JSON.stringify(was.parts)) {
          problems.push(`extent ${index}: the requests differ from the committed fixture`);
          return;
        }
        const differs =
          Math.abs(entry.x - was.x) > EXTENT_TOLERANCE ||
          Math.abs(entry.width - was.width) > EXTENT_TOLERANCE ||
          Math.abs(entry.computedLength - was.computedLength) > EXTENT_TOLERANCE;
        if (differs) {
          problems.push(
            `extent ${JSON.stringify(entry.parts)} ${entry.size} ${entry.anchor}: ` +
              `${entry.x}/${entry.width} vs ${was.x}/${was.width}`
          );
        }
      });
      measured.fallback.forEach((entry, index) => {
        const was = committed.fallback[index];
        if (was === undefined || JSON.stringify(entry.parts) !== JSON.stringify(was.parts)) {
          problems.push(`fallback ${index}: the requests differ from the committed fixture`);
          return;
        }
        const differs =
          Math.abs(entry.x - was.x) > EXTENT_TOLERANCE ||
          Math.abs(entry.width - was.width) > EXTENT_TOLERANCE ||
          Math.abs(entry.computedLength - was.computedLength) > EXTENT_TOLERANCE ||
          entry.ascent !== was.ascent ||
          entry.descent !== was.descent;
        if (differs) {
          problems.push(
            `fallback ${JSON.stringify(entry.parts)} ${entry.size} ${entry.anchor}: ` +
              `${entry.x}/${entry.width}/${entry.ascent}/${entry.descent} vs ` +
              `${was.x}/${was.width}/${was.ascent}/${was.descent}`
          );
        }
      });
      measured.fallbackWraps.forEach((entry, index) => {
        const was = committed.fallbackWraps[index];
        if (was === undefined || JSON.stringify(entry.lines) !== JSON.stringify(was.lines)) {
          problems.push(
            `fallback wrap ${entry.text} at ${entry.wrapWidth}: ${JSON.stringify(entry.lines)}`
          );
        }
      });
      if (problems.length > 0) {
        console.error(problems.join('\n'));
        console.error(`${problems.length} differences from ${fixturePath}`);
        process.exit(1);
      }
      console.log(
        `chromium fixture: ${measured.widths.length} widths, ${measured.runs.length} runs, ` +
          `${measured.wraps.length} wraps, ${measured.boxes.length} boxes, ${measured.extents.length} extents, ` +
          `${measured.fallback.length} fallback lines and ${measured.fallbackWraps.length} fallback wraps ` +
          'agree with the committed values'
      );
      return;
    }

    await writeFile(fixturePath, formatFixture(measured));
    console.log(`wrote ${fixturePath}`);
  } finally {
    await browser.close();
  }
}

await main();
