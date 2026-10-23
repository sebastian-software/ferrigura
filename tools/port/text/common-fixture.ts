/**
 * Regenerates `crates/ferrigura-render/tests/fixtures/common-chromium.json`: what the reference's
 * shared helpers of the custom renderers give in Chromium, for the requests listed below.
 *
 * The helpers are the reference's own code. `utils.ts` (`calculateTextDimensions`, `wrapLabel`,
 * `calculateTextWidth`, `calculateTextHeight`, `drawSimpleText`, `insertTitle`), its unexported
 * `breakString` (exposed by appending one `export` line to the module as it is loaded, nothing else
 * changed), `diagrams/common/svgDrawCommon.ts` and `setupGraphViewbox.js` are bundled with esbuild
 * and run in the page, as `label-fixture.ts` does for `createText`. The page loads Lexica
 * Ultralegible as a web font before anything is measured. The configured font family is the one
 * the harness forces (`"Lexica Ultralegible"`).
 *
 * The page keeps Mermaid's memoisation: `calculateTextDimensions` and `wrapLabel` cache by the key
 * the reference gives (`${text}${fontSize}${fontWeight}${fontFamily}` and the wrap key), and the
 * cache lives as long as the page. The requests therefore run in a page of their own whenever two
 * of them could share a key, and the probe fails if two requests share a key in one page. The
 * memoisation itself is measured with two requests that do collide (`memo` in the fixture).
 *
 * Each `dimensions` entry records the reference's result for Lexica and the result for
 * `sans-serif`, the family that the reference also measures (`calculateTextDimensions` keeps the
 * sans-serif values only when they are larger in all three of width, height and line height).
 *
 * Usage: `pnpm exec tsx tools/port/text/common-fixture.ts [--check]`. With `--check`, the committed
 * fixture is compared with a fresh run and the script exits non-zero on a difference larger than
 * the tolerance.
 */
// cspell:ignore Straße AVAV Ünïcödé wörds çedillas Pneumono
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { build, type Plugin } from 'esbuild';
import { chromium, type Browser, type Page } from 'playwright';

import { referenceRoot } from '../reference-root.ts';

// The reference's esbuild plugin for its configuration schema, from its checkout.
const { default: jsonSchemaPlugin } = (await import(
  pathToFileURL(join(referenceRoot, '.esbuild/jsonSchemaPlugin.ts')).href
)) as { default: Plugin };

const root = fileURLToPath(new URL('../../../', import.meta.url));
const fixturePath = join(root, 'crates/ferrigura-render/tests/fixtures/common-chromium.json');
const fontDir = join(root, 'crates/ferrigura-text/fonts');
const sourceDir = join(referenceRoot, 'packages/mermaid/src');
const utilsPath = join(sourceDir, 'utils.ts');
const svgDrawCommonPath = join(sourceDir, 'diagrams/common/svgDrawCommon.ts');
const viewboxPath = join(sourceDir, 'setupGraphViewbox.js');

/** The tolerance of measured numbers, in pixels, when `--check` compares fresh values. */
const TOLERANCE = 0.0001;

const FAMILY = '"Lexica Ultralegible"';
const FACE_FILES: [number, string, string][] = [
  [400, 'normal', 'LexicaUltralegible-Regular.ttf'],
  [400, 'italic', 'LexicaUltralegible-Italic.ttf'],
  [700, 'normal', 'LexicaUltralegible-Bold.ttf'],
  [700, 'italic', 'LexicaUltralegible-BoldItalic.ttf'],
];

interface Dimensions {
  text: string;
  fontSize: number;
  fontWeight: number;
  fontFamily: string;
}

interface Outcome<T> {
  value?: T;
  error?: string;
}

interface DimensionResult extends Dimensions {
  lexica: Outcome<{ width: number; height: number; lineHeight?: number }>;
  lexicaWidth?: number;
  lexicaHeight?: number;
  sans: Outcome<{ width: number; height: number; lineHeight?: number }>;
}

interface WrapRequest {
  label: string;
  maxWidth: number;
  fontSize: number;
  fontWeight: number;
  joinWith: string;
}

interface BreakRequest {
  word: string;
  maxWidth: number;
  fontSize: number;
  fontWeight: number;
}

interface WrapResult extends WrapRequest {
  fontFamily: string;
  lines: Outcome<string>;
}

interface BreakResult extends BreakRequest {
  fontFamily: string;
  result: Outcome<{ hyphenatedStrings: string[]; remainingWord: string }>;
}

/** The sizes, weights and texts of the dimension requests. */
const TEXTS: string[] = [
  'Hello',
  'Hello world',
  'A plain label with several words',
  'The quick brown fox jumps over the lazy dog',
  'Supercalifragilisticexpialidocious',
  'Pneumonoultramicroscopicsilicovolcanoconiosis',
  'a<br>b',
  'a<br/>b',
  'a<br />b',
  'a<BR>b',
  'a</br>b',
  'a<br/>',
  '<br/>a',
  'a<br/><br/>b',
  '<br/><br/>',
  'a<br>b<br>c',
  'one<br/>two words<br/>three',
  '',
  ' ',
  '   ',
  ' a ',
  'a  b',
  'a\tb',
  'a\nb',
  'trailing ',
  ' leading',
  '12345',
  '3.14159',
  '1,000,000',
  '0',
  '-',
  '2026-10-16',
  'Hello, world! (test) [a] {b} "c" \'d\' & e',
  'a < b > c & d',
  'x<y>z',
  'Café naïve Straße',
  'ﬂ°°35¶ß',
  'WAVE To AVAV',
  'iiii WWWW',
  'Mm',
  'ÅÄÖ åäö',
  '&amp; &lt;',
  'it&#39;s',
  // Ink past the advance: `getBBox` takes the glyph outlines of a leading `j` and a trailing `q`
  // into the box, so these widths are the boxes, not the advances.
  'j',
  'jun',
  'jul',
  'jq',
  'Jj gq',
  'fjord',
  'quit',
];
const SIZES = [10, 12, 12.5, 14, 16, 16.25, 20, 24, 32];
const WEIGHTS = [300, 400, 600, 700];

const LABELS: string[] = [
  'The quick brown fox jumps over the lazy dog',
  'Supercalifragilisticexpialidocious and more words',
  'Pneumonoultramicroscopicsilicovolcanoconiosis',
  'one two three four five six',
  'a<br/>b tail words',
  'x<br />y',
  '',
  '   ',
  'Ünïcödé wörds with çedillas and more',
  'two  spaces   in  it',
  'trailing space ',
  ' leading space',
  'tab\there is long enough to wrap around the width',
  'ab',
  'aaaa bbbb cccc',
  'Mixed: punctuation, commas; and (brackets) here',
  '1234567890 1234567890 1234567890',
];
const WRAP_WIDTHS = [20, 60, 120, 150, 200];
const WRAP_SIZES = [12, 16, 20];

const BREAKS: [string, number][] = [
  ['Supercalifragilisticexpialidocious', 100],
  ['abcdefghij', 30],
  ['Pneumono', 50],
  ['mmmmmm', 40],
  ['iiiiiiii', 20],
  ['a', 1],
  ['', 10],
  ['Café', 3],
  ['ab\tcd', 25],
  ['WWWWWW', 60],
];

/** The page's code: the reference's helpers, bundled from their source, and the probe functions. */
const entry = `
import { select } from 'd3';
import {
  calculateTextDimensions,
  calculateTextWidth,
  calculateTextHeight,
  drawSimpleText,
  getTextObj as getUtilsTextObj,
  wrapLabel,
  breakString,
  insertTitle,
} from ${JSON.stringify(utilsPath)};
import {
  drawRect,
  drawBackgroundRect,
  drawText,
  drawImage,
  drawEmbeddedImage,
  getNoteRect,
  getTextObj,
} from ${JSON.stringify(svgDrawCommonPath)};
import {
  setupGraphViewbox,
  configureSvgSize,
  calculateSvgSizeAttrs,
} from ${JSON.stringify(viewboxPath)};

const SVG = 'http://www.w3.org/2000/svg';

const outcome = (run) => {
  try {
    return { value: run() };
  } catch (error) {
    return { error: String(error && error.message ? error.message : error) };
  }
};

// The markup of an element, in the attribute order it was given: the form the Rust side writes.
const escapeText = (text) =>
  text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
const escapeAttribute = (text) => escapeText(text).replace(/"/g, '&quot;').replace(/'/g, '&#39;');
const XLINK = 'http://www.w3.org/1999/xlink';
const markup = (element) => {
  const attributes = [...element.attributes]
    .map((attribute) => {
      // d3 sets xlink:href in the xlink namespace; the serialised form carries the prefix.
      const name = attribute.namespaceURI === XLINK ? 'xlink:' + attribute.localName : attribute.name;
      return ' ' + name + '="' + escapeAttribute(attribute.value) + '"';
    })
    .join('');
  const children = [...element.childNodes]
    .map((child) => (child.nodeType === 3 ? escapeText(child.textContent) : markup(child)))
    .join('');
  return children === ''
    ? '<' + element.tagName + attributes + '/>'
    : '<' + element.tagName + attributes + '>' + children + '</' + element.tagName + '>';
};
const newSvg = () => {
  const svg = document.createElementNS(SVG, 'svg');
  svg.setAttribute('width', '1000');
  svg.setAttribute('height', '1000');
  document.body.appendChild(svg);
  return svg;
};

window.dimensions = (requests) =>
  requests.map((request) => {
    const config = { fontSize: request.fontSize, fontWeight: request.fontWeight, fontFamily: request.fontFamily };
    const lexica = outcome(() => calculateTextDimensions(request.text, config));
    const width = outcome(() => calculateTextWidth(request.text, config));
    const height = outcome(() => calculateTextHeight(request.text, config));
    const sans = outcome(() =>
      calculateTextDimensions(request.text, { ...config, fontFamily: 'sans-serif' })
    );
    return { ...request, lexica, widthOnly: width, heightOnly: height, sans };
  });

window.wrap = (request) =>
  outcome(() =>
    wrapLabel(request.label, request.maxWidth, {
      fontSize: request.fontSize,
      fontWeight: request.fontWeight,
      fontFamily: request.fontFamily,
      joinWith: request.joinWith,
    })
  );

window.breakWord = (request) =>
  outcome(() =>
    breakString(request.word, request.maxWidth, '-', {
      fontSize: request.fontSize,
      fontWeight: request.fontWeight,
      fontFamily: request.fontFamily,
      joinWith: '<br/>',
    })
  );

window.insertTitleCase = (title, margin, cssClass) => {
  const svg = newSvg();
  const parent = select(svg).append('g');
  parent.append('rect').attr('x', 10).attr('y', 5).attr('width', 100).attr('height', 40);
  const bounds = parent.node().getBBox();
  insertTitle(parent, cssClass, margin, title);
  const result = { bounds: { x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height }, markup: markup(parent.node()) };
  svg.remove();
  return result;
};

window.drawingCases = () => {
  const out = {};
  const svg = newSvg();
  const drawn = (name, run) => {
    const target = select(svg).append('g');
    run(target);
    out[name] = markup(target.node());
    target.remove();
  };
  drawn('rect-plain', (g) => drawRect(g, { x: 10, y: 20, fill: '#fff', stroke: '#000', width: 100, height: 50 }));
  drawn('rect-all', (g) =>
    drawRect(g, {
      x: 0,
      y: 0,
      width: 10,
      height: 10,
      fill: 'red',
      stroke: 'none',
      rx: 5,
      ry: 5,
      class: 'rect',
      name: 'n1',
      attrs: { 'data-x': '1', fill: 'blue' },
    })
  );
  drawn('rect-zero-radii', (g) =>
    drawRect(g, { x: 1, y: 2, width: 3, height: 4, fill: 'none', stroke: 'none', rx: 0, ry: 0 })
  );
  drawn('rect-undefined-fill', (g) =>
    drawRect(g, { x: 1.5, y: -2, width: 0.1, height: 1e21, fill: undefined, stroke: undefined })
  );
  drawn('rect-numbers', (g) =>
    drawRect(g, { x: 0.1 + 0.2, y: 1e-7, width: 1 / 3, height: 123456789.5, fill: '#abc', stroke: '#def' })
  );
  drawn('background', (g) => {
    g.append('text').attr('x', 1).text('before');
    drawBackgroundRect(g, { startx: 5, starty: 6, stopx: 105, stopy: 56, fill: '#eee', stroke: 'none' });
  });
  drawn('text-middle', (g) =>
    drawText(g, { x: 10, y: 20, text: 'a<br/>b', anchor: 'middle', textMargin: 5, class: undefined })
  );
  drawn('text-class', (g) =>
    drawText(g, { x: 0, y: 0, text: 'Hi <b>there</b>', anchor: 'start', textMargin: 0, class: 'legend-x' })
  );
  drawn('text-no-anchor', (g) => drawText(g, { x: 3, y: 4, text: 'x', anchor: undefined, textMargin: 2 }));
  drawn('text-breaks', (g) =>
    drawText(g, { x: 1, y: 2, text: 'one<BR>two<br />three', anchor: 'end', textMargin: 0 })
  );
  drawn('image', (g) => drawImage(g, 1, 2, 'https://example.com/a.png?x=1&y=2'));
  drawn('image-unsafe', (g) => drawImage(g, 1, 2, 'javascript:alert(1)'));
  drawn('image-relative', (g) => drawImage(g, 1, 2, 'pics/a b.png'));
  drawn('embedded', (g) => drawEmbeddedImage(g, 3, 4, 'my-id'));
  drawn('embedded-unsafe', (g) => drawEmbeddedImage(g, 3, 4, 'javascript:alert(1)'));
  out.noteRect = getNoteRect();
  out.textObject = getTextObj();
  svg.remove();
  return out;
};

window.viewboxCase = (padding, useMaxWidth) => {
  const svg = newSvg();
  svg.removeAttribute('height');
  svg.removeAttribute('width');
  const group = select(svg).append('g');
  group.append('rect').attr('x', 5).attr('y', 7).attr('width', 120.5).attr('height', 33.25);
  const text = group.append('text').attr('x', 10).attr('y', 60).text('Hello');
  const bounds = svg.getBBox();
  setupGraphViewbox(undefined, select(svg), padding, useMaxWidth);
  const result = {
    bounds: { x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height },
    markup: markup(svg),
  };
  text.remove();
  svg.remove();
  return result;
};

window.sizeCase = (height, width, useMaxWidth) => {
  const svg = newSvg();
  configureSvgSize(select(svg), height, width, useMaxWidth);
  const attributes = [...calculateSvgSizeAttrs(height, width, useMaxWidth).entries()];
  const result = { attributes, markup: markup(svg) };
  svg.remove();
  return result;
};

window.sanitize = (value) => {
  const svg = newSvg();
  drawImage(select(svg), 0, 0, value);
  const element = svg.firstChild;
  const href = element.getAttributeNS('http://www.w3.org/1999/xlink', 'href');
  svg.remove();
  return href;
};

window.rawBox = (text, fontSize, fontWeight) => {
  // The element calculateTextDimensions measures, drawn by the reference's own drawSimpleText.
  const svg = newSvg();
  const el = drawSimpleText(select(svg), {
    ...getUtilsTextObj(),
    text,
    fontFamily: '"Lexica Ultralegible"',
    fontSize,
    fontWeight,
  });
  const node = el.node();
  const box = node.getBBox();
  const result = {
    x: box.x,
    y: box.y,
    width: box.width,
    height: box.height,
    computedLength: node.getComputedTextLength(),
  };
  svg.remove();
  return result;
};

window.ready = true;
`;

/** Exposes `breakString`, which the module does not export: one `export` line, nothing else. */
const exposeBreakString: Plugin = {
  name: 'expose-break-string',
  setup(buildContext) {
    buildContext.onLoad({ filter: /utils\.ts$/ }, async (args) => {
      if (args.path !== utilsPath) return undefined;
      const contents = await readFile(args.path, 'utf8');
      return { contents: `${contents}\nexport { breakString };\n`, loader: 'ts' };
    });
  },
};

/** Bundles the entry with the reference's helpers and the modules they import. */
async function bundle(): Promise<string> {
  const result = await build({
    stdin: {
      contents: entry,
      resolveDir: sourceDir,
      sourcefile: 'common-entry.js',
      loader: 'js',
    },
    bundle: true,
    format: 'iife',
    platform: 'browser',
    target: 'es2022',
    write: false,
    logLevel: 'silent',
    plugins: [exposeBreakString, jsonSchemaPlugin],
    // Build-time flags of the Mermaid bundle: the profiler is off in production.
    define: { injected: '{"version":"fixture","profiling":false}' },
  });
  return result.outputFiles[0].text;
}

/** A page with the four faces loaded and the bundle run. */
async function openPage(browser: Browser, script: string): Promise<Page> {
  const faces = await Promise.all(
    FACE_FILES.map(async ([weight, style, file]) => {
      const data = (await readFile(join(fontDir, file))).toString('base64');
      return `@font-face { font-family: "Lexica Ultralegible"; font-weight: ${weight}; font-style: ${style}; src: url(data:font/ttf;base64,${data}) format("truetype"); }`;
    })
  );
  const page = await browser.newPage();
  await page.setContent(
    `<!doctype html><html><head><meta charset="utf-8"><style>${faces.join('\n')}\nbody { margin: 0; }</style></head><body></body></html>`
  );
  await page.evaluate(async () => {
    for (const font of ['normal 16px', 'italic 16px', 'bold 16px', 'italic bold 16px']) {
      await document.fonts.load(`${font} "Lexica Ultralegible"`);
    }
    await document.fonts.ready;
  });
  await page.addScriptTag({ content: script });
  return page;
}

/** The memoisation key of `calculateTextDimensions`, as the reference writes it. */
const dimensionsKey = (request: Dimensions, family: string): string =>
  `${request.text}${request.fontSize}${request.fontWeight}${family}`;

/** Two requests that share a key in one page would share a cached result: the probe refuses. */
function checkDimensionKeys(requests: Dimensions[]): void {
  const seen = new Map<string, string>();
  for (const request of requests) {
    for (const family of [request.fontFamily, 'sans-serif']) {
      const key = dimensionsKey(request, family);
      const tuple = JSON.stringify([request.text, request.fontSize, request.fontWeight, family]);
      const earlier = seen.get(key);
      if (earlier !== undefined && earlier !== tuple) {
        throw new Error(`memo key ${JSON.stringify(key)} shared by ${earlier} and ${tuple}`);
      }
      seen.set(key, tuple);
    }
  }
}

interface Collision {
  first: Dimensions;
  second: Dimensions;
}

interface Fixture {
  chromium: string;
  family: string;
  dimensions: DimensionResult[];
  wraps: WrapResult[];
  breaks: BreakResult[];
  memo: {
    dimensions: {
      first: Dimensions;
      second: Dimensions;
      inPage: Outcome<DimensionResult['lexica']['value']>;
      alone: Outcome<DimensionResult['lexica']['value']>;
    }[];
    wrap: {
      first: WrapRequest;
      second: WrapRequest;
      inPage: Outcome<string>;
      alone: Outcome<string>;
    }[];
  };
  drawing: Record<string, unknown>;
  insertTitle: {
    title: string;
    margin: number;
    cssClass: string;
    bounds: unknown;
    markup: string;
  }[];
  viewbox: { padding: number; useMaxWidth: boolean; bounds: unknown; markup: string }[];
  sizes: {
    height: number;
    width: number;
    useMaxWidth: boolean;
    attributes: [string, string | number][];
    markup: string;
  }[];
  sanitize: { input: string; href: string | null }[];
  boxes: { text: string; fontSize: number; fontWeight: number; box: unknown }[];
}

const DIMENSION_REQUESTS: Dimensions[] = TEXTS.flatMap((text) => [
  ...SIZES.map((fontSize) => ({ text, fontSize, fontWeight: 400, fontFamily: FAMILY })),
  ...WEIGHTS.filter((fontWeight) => fontWeight !== 400).map((fontWeight) => ({
    text,
    fontSize: 16,
    fontWeight,
    fontFamily: FAMILY,
  })),
]);

const WRAP_REQUESTS: WrapRequest[] = LABELS.flatMap((label) =>
  WRAP_WIDTHS.flatMap((maxWidth) =>
    WRAP_SIZES.map((fontSize) => ({
      label,
      maxWidth,
      fontSize,
      fontWeight: 400,
      joinWith: '<br/>',
    }))
  )
);

const BREAK_REQUESTS: BreakRequest[] = BREAKS.flatMap(([word, maxWidth]) =>
  [12, 16].map((fontSize) => ({ word, maxWidth, fontSize, fontWeight: 400 }))
);

const VIEWBOX_CASES: [number, boolean][] = [
  [8, true],
  [0, false],
  [5.5, true],
];
const SIZE_CASES: [number, number, boolean][] = [
  [100, 400, true],
  [450, 300.5, false],
  [0, 0, true],
];
const TITLE_CASES: [string, number, string][] = [
  ['My title', 25, 'gitTitleText'],
  ['Title with <b> & "quotes"', 0, 'title'],
];
const SANITIZE_INPUTS = [
  'https://example.com/a.png?x=1&y=2',
  'javascript:alert(1)',
  'data:image/png;base64,AAAA',
  'pics/a b.png',
  ' https://example.com/space.png ',
  'HTTPS://EXAMPLE.COM/UPPER',
  'mailto:someone@example.com',
];
const RAW_TEXTS = TEXTS.filter((text) => text !== '' && text.trim() !== '');
const RAW_SIZES = [10, 10.005, 12.5, 13.337, 16, 16.25, 20, 32];
const RAW_WEIGHTS = [400, 700];

async function measureAll(): Promise<Fixture> {
  checkDimensionKeys(DIMENSION_REQUESTS);
  const browser = await chromium.launch({
    // Without hinting Chromium keeps fractional advances, as it does on macOS.
    args: ['--font-render-hinting=none'],
  });
  try {
    const script = await bundle();
    const chromiumVersion = browser.version();

    const page = await openPage(browser, script);
    const dimensions = (await page.evaluate(
      (requests: Dimensions[]) =>
        (window as unknown as { dimensions: (r: Dimensions[]) => unknown }).dimensions(requests),
      DIMENSION_REQUESTS
    )) as DimensionResult[];
    await page.close();

    const wraps: WrapResult[] = [];
    for (const request of WRAP_REQUESTS) {
      const fresh = await openPage(browser, script);
      const lines = (await fresh.evaluate(
        (input: WrapRequest & { fontFamily: string }) =>
          (window as unknown as { wrap: (r: unknown) => unknown }).wrap(input),
        { ...request, fontFamily: FAMILY }
      )) as Outcome<string>;
      wraps.push({ ...request, fontFamily: FAMILY, lines });
      await fresh.close();
    }

    const breaks: BreakResult[] = [];
    for (const request of BREAK_REQUESTS) {
      const fresh = await openPage(browser, script);
      const result = (await fresh.evaluate(
        (input: BreakRequest & { fontFamily: string }) =>
          (window as unknown as { breakWord: (r: unknown) => unknown }).breakWord(input),
        { ...request, fontFamily: FAMILY }
      )) as BreakResult['result'];
      breaks.push({ ...request, fontFamily: FAMILY, result });
      await fresh.close();
    }

    // Memoisation: a second request with the same key as an earlier one returns the earlier result.
    const memoDimensions: Collision[] = [
      {
        first: { text: 'a1', fontSize: 6, fontWeight: 400, fontFamily: FAMILY },
        second: { text: 'a', fontSize: 16, fontWeight: 400, fontFamily: FAMILY },
      },
    ];
    const memo: Fixture['memo'] = { dimensions: [], wrap: [] };
    for (const pair of memoDimensions) {
      const inPage = await openPage(browser, script);
      const results = (await inPage.evaluate(
        (requests: Dimensions[]) =>
          (window as unknown as { dimensions: (r: Dimensions[]) => unknown }).dimensions(requests),
        [pair.first, pair.second]
      )) as DimensionResult[];
      const alone = await openPage(browser, script);
      const [aloneResult] = (await alone.evaluate(
        (requests: Dimensions[]) =>
          (window as unknown as { dimensions: (r: Dimensions[]) => unknown }).dimensions(requests),
        [pair.second]
      )) as DimensionResult[];
      memo.dimensions.push({
        first: pair.first,
        second: pair.second,
        inPage: results[1].lexica,
        alone: aloneResult.lexica,
      });
      await inPage.close();
      await alone.close();
    }
    const memoWraps: { first: WrapRequest; second: WrapRequest }[] = [
      {
        first: { label: 'ab', maxWidth: 120, fontSize: 16, fontWeight: 400, joinWith: '<br/>' },
        second: { label: 'ab1', maxWidth: 20, fontSize: 16, fontWeight: 400, joinWith: '<br/>' },
      },
    ];
    for (const pair of memoWraps) {
      const inPage = await openPage(browser, script);
      await inPage.evaluate(
        (input: WrapRequest & { fontFamily: string }) =>
          (window as unknown as { wrap: (r: unknown) => unknown }).wrap(input),
        { ...pair.first, fontFamily: FAMILY }
      );
      const inPageResult = (await inPage.evaluate(
        (input: WrapRequest & { fontFamily: string }) =>
          (window as unknown as { wrap: (r: unknown) => unknown }).wrap(input),
        { ...pair.second, fontFamily: FAMILY }
      )) as Outcome<string>;
      const alone = await openPage(browser, script);
      const aloneResult = (await alone.evaluate(
        (input: WrapRequest & { fontFamily: string }) =>
          (window as unknown as { wrap: (r: unknown) => unknown }).wrap(input),
        { ...pair.second, fontFamily: FAMILY }
      )) as Outcome<string>;
      memo.wrap.push({ ...pair, inPage: inPageResult, alone: aloneResult });
      await inPage.close();
      await alone.close();
    }

    const drawingPage = await openPage(browser, script);
    const drawing = (await drawingPage.evaluate(() =>
      (window as unknown as { drawingCases: () => unknown }).drawingCases()
    )) as Record<string, unknown>;
    const insertTitles: Fixture['insertTitle'] = [];
    const viewbox: Fixture['viewbox'] = [];
    const sizes: Fixture['sizes'] = [];
    const sanitize: Fixture['sanitize'] = [];
    const boxes: Fixture['boxes'] = [];
    for (const [title, margin, cssClass] of TITLE_CASES) {
      const result = (await drawingPage.evaluate(
        ([t, m, c]: [string, number, string]) =>
          (
            window as unknown as { insertTitleCase: (a: string, b: number, c: string) => unknown }
          ).insertTitleCase(t, m, c),
        [title, margin, cssClass] as [string, number, string]
      )) as { bounds: unknown; markup: string };
      insertTitles.push({ title, margin, cssClass, ...result });
    }
    for (const [padding, useMaxWidth] of VIEWBOX_CASES) {
      const result = (await drawingPage.evaluate(
        ([p, u]: [number, boolean]) =>
          (window as unknown as { viewboxCase: (a: number, b: boolean) => unknown }).viewboxCase(
            p,
            u
          ),
        [padding, useMaxWidth] as [number, boolean]
      )) as { bounds: unknown; markup: string };
      viewbox.push({ padding, useMaxWidth, ...result });
    }
    for (const [height, width, useMaxWidth] of SIZE_CASES) {
      const result = (await drawingPage.evaluate(
        ([h, w, u]: [number, number, boolean]) =>
          (
            window as unknown as { sizeCase: (a: number, b: number, c: boolean) => unknown }
          ).sizeCase(h, w, u),
        [height, width, useMaxWidth] as [number, number, boolean]
      )) as { attributes: [string, string | number][]; markup: string };
      sizes.push({ height, width, useMaxWidth, ...result });
    }
    for (const input of SANITIZE_INPUTS) {
      const href = (await drawingPage.evaluate(
        (value: string) =>
          (window as unknown as { sanitize: (v: string) => unknown }).sanitize(value),
        input
      )) as string | null;
      sanitize.push({ input, href });
    }
    for (const text of RAW_TEXTS) {
      for (const fontSize of RAW_SIZES) {
        for (const fontWeight of RAW_WEIGHTS) {
          const box = (await drawingPage.evaluate(
            ([value, size, weight]: [string, number, number]) =>
              (
                window as unknown as { rawBox: (a: string, b: number, c: number) => unknown }
              ).rawBox(value, size, weight),
            [text, fontSize, fontWeight] as [string, number, number]
          )) as unknown;
          boxes.push({ text, fontSize, fontWeight, box });
        }
      }
    }
    await drawingPage.close();

    return {
      chromium: chromiumVersion,
      family: FAMILY,
      dimensions,
      wraps,
      breaks,
      memo,
      drawing,
      insertTitle: insertTitles,
      viewbox,
      sizes,
      sanitize,
      boxes,
    };
  } finally {
    await browser.close();
  }
}

/** The differences between a fresh run and the committed fixture, as readable lines. */
function compare(fresh: unknown, committed: unknown, where: string, problems: string[]): void {
  if (typeof fresh === 'number' && typeof committed === 'number') {
    if (Math.abs(fresh - committed) > TOLERANCE)
      problems.push(`${where}: ${fresh} vs ${committed}`);
    return;
  }
  if (Array.isArray(fresh) && Array.isArray(committed)) {
    if (fresh.length !== committed.length) {
      problems.push(`${where}: ${fresh.length} entries vs ${committed.length}`);
      return;
    }
    fresh.forEach((value, index) =>
      compare(value, committed[index], `${where}[${index}]`, problems)
    );
    return;
  }
  if (fresh && committed && typeof fresh === 'object' && typeof committed === 'object') {
    const keys = new Set([...Object.keys(fresh), ...Object.keys(committed)]);
    for (const key of keys) {
      compare(
        (fresh as Record<string, unknown>)[key],
        (committed as Record<string, unknown>)[key],
        `${where}.${key}`,
        problems
      );
    }
    return;
  }
  if (fresh !== committed)
    problems.push(`${where}: ${JSON.stringify(fresh)} vs ${JSON.stringify(committed)}`);
}

async function main(): Promise<void> {
  const check = process.argv.includes('--check');
  const fresh = await measureAll();
  if (check) {
    const committed = JSON.parse(await readFile(fixturePath, 'utf8')) as unknown;
    const problems: string[] = [];
    compare(fresh, committed, 'fixture', problems);
    if (problems.length > 0) {
      console.error(`${problems.length} differences from the committed fixture:`);
      console.error(problems.slice(0, 40).join('\n'));
      process.exit(1);
    }
    console.log('the committed fixture matches a fresh run');
    return;
  }
  await writeFile(fixturePath, `${JSON.stringify(fresh, null, 2)}\n`);
  console.log(`wrote ${fixturePath}`);
  console.log(
    `${fresh.dimensions.length} dimension requests, ${fresh.wraps.length} wraps, ${fresh.breaks.length} breaks`
  );
}

await main();
