/**
 * Regenerates `crates/ferrigura-render/tests/fixtures/labels-chromium.json`: what the reference's
 * `createText` (`packages/mermaid/src/rendering-util/createText.ts`, `htmlLabels: false`) draws in
 * Chromium for the labels listed below.
 *
 * The reference is run from its source: `createText` is bundled with esbuild together with the
 * modules it imports, and the bundle runs in the page. The bundle is the same code the built
 * Mermaid runs, but `createText` is not exported from the built `mermaid.esm.mjs`, so the source is
 * bundled instead. The page loads Lexica Ultralegible (the four bundled styles) as a web font, and
 * its root carries the font size of the request, as the flowchart's root CSS does (14 px in the
 * default theme). The `.node .label text` rule of the flowchart stylesheet is copied in, since
 * node labels are centred by it.
 *
 * Each label is measured the way the reference calls it: node labels inside the `g.label` that
 * `labelHelper` creates, with that group's `style` attribute, edge labels in the plain group that
 * `insertEdgeLabel` passes, and cluster titles in a `g.cluster-label` as `clusters.js` does. The recorded values are, for every outer line and inner word:
 * the text (with its leading space), the computed weight, style and font size, and the start
 * position of its first character. Also recorded: each line's `getComputedTextLength`, the
 * `getBBox` of the `<text>`, and the background rectangle when there is one.
 *
 * Chromium is the reference on purpose. WebKit reports widths without kerning through the SVG text
 * API, so no check here runs in WebKit (`docs-port/spikes/text-measurement.md`).
 *
 * Usage: `pnpm exec tsx tools/port/text/label-fixture.ts [--check]`. With `--check`, the committed
 * fixture is compared with a fresh run and the script exits non-zero on a difference larger than
 * the tolerance.
 */
import { readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { build } from 'esbuild';
import { chromium } from 'playwright';

import jsonSchemaPlugin from '../../../.esbuild/jsonSchemaPlugin.js';
import { referenceRoot } from '../reference-root.ts';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const fixturePath = join(root, 'crates/ferrigura-render/tests/fixtures/labels-chromium.json');
const fontDir = join(root, 'crates/ferrigura-text/fonts');
const createTextPath = join(referenceRoot, 'packages/mermaid/src/rendering-util/createText.ts');

/** The tolerance of positions and widths, in pixels, when `--check` compares fresh values. */
const TOLERANCE = 0.001;

/** One label of the fixture: the arguments of the reference's `createText` in its caller's setup. */
interface Request {
  id: string;
  text: string;
  /** The `labelType` of the label: markdown takes the Markdown path, text and string the plain one. */
  labelType: 'text' | 'string' | 'markdown';
  /** Node labels are inside `labelHelper`'s group; edge labels take `insertEdgeLabel`'s call. */
  isNode: boolean;
  /**
   * Where the lines are anchored: `middle` for node and edge labels (the stylesheet or the attribute
   * centres them), `start` for a cluster title, which has no `text-anchor`.
   */
  anchor: 'middle' | 'start';
  /** The wrap width, or `null` for the reference's default (120 for nodes, 200 for edges). */
  width: number | null;
  /** The label's style declarations, as `styles2String` writes them. */
  style: string;
  /** Whether the background is drawn (`addSvgBackground`). Edge labels always draw it. */
  addBackground: boolean;
  /** The root font size of the diagram, in pixels. */
  fontSize: number;
}

interface Word {
  text: string;
  weight: number;
  style: string;
  fontSize: number;
  x: number;
  y: number;
}

interface Box {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface Measured extends Request {
  /** The reference's error, when it cannot lay the label out; the other fields are then absent. */
  error?: string;
  lines: Word[][];
  lineTextLengths: number[];
  bbox: Box;
  background: Box | null;
}

/** A wrap width that no label reaches: the reference's `Infinity` for a plain cluster title. */
const NO_WRAP = 1e9;

const node = (
  id: string,
  text: string,
  labelType: Request['labelType'],
  extra: Partial<Request> = {}
): Request => ({
  id,
  text,
  labelType,
  isNode: true,
  anchor: 'middle',
  width: 120,
  style: '',
  addBackground: false,
  fontSize: 16,
  ...extra,
});

const edge = (
  id: string,
  text: string,
  labelType: Request['labelType'],
  extra: Partial<Request> = {}
): Request => ({
  id,
  text,
  labelType,
  isNode: false,
  anchor: 'middle',
  width: null,
  style: '',
  addBackground: true,
  fontSize: 16,
  ...extra,
});

/**
 * The labels of the fixture. Plain labels (`text`, `string`) take `nonMarkdownToLines`, Markdown
 * labels take `markdownToLines`. Widths are the ones the reference uses at the call sites: 120 for
 * nodes, the default 200 for edges, and 120 and 200 as explicit requests to see both wraps.
 */
const requests: Request[] = [
  // Plain labels: breaks, whitespace, brackets, entities.
  node('plain-start', 'Start', 'text'),
  node('plain-words', 'A plain label with several words', 'text'),
  node('plain-spaces', 'two  spaces   and single', 'text'),
  node('plain-br', 'a<br>b', 'text'),
  node('plain-br-slash', 'a<br/>b', 'text'),
  node('plain-br-space', 'a<br />b', 'text'),
  node('plain-br-upper', 'a<BR>b', 'text'),
  node('plain-backslash-n', 'a\\nb', 'text'),
  node('plain-newline', 'a\nb', 'text'),
  node('plain-trailing-br', 'trailing<br/>', 'text'),
  node('plain-leading-br', '<br/>leading', 'text'),
  node('plain-empty-middle', 'a<br/><br/>b', 'text'),
  node('plain-only-breaks', '<br/><br/>', 'text'),
  node('plain-empty', '', 'text'),
  node('plain-blank', '   ', 'text'),
  node('plain-brackets', 'x < y and a > b', 'text'),
  node('plain-tags-split', 'x<y>z', 'text'),
  node('plain-entities', 'it&#39;s &amp; co', 'text'),
  node('plain-entity-names', 'a &lt;c&gt; d', 'text'),
  node('plain-placeholder', 'x ﬂ°°35¶ß y', 'text'),
  node('plain-html-tags', '<b>bold</b> text', 'text'),
  node('plain-quotes', 'say "hi" to \'you\'', 'text'),
  node(
    'plain-long-120',
    'The quick brown fox jumps over the lazy dog and keeps running far away',
    'text'
  ),
  node('plain-long-word-120', 'Supercalifragilisticexpialidocious and more words', 'text'),
  node('plain-long-word-200', 'Pneumonoultramicroscopicsilicovolcanoconiosis', 'text', {
    width: 200,
  }),
  node('plain-14px', 'One line label', 'text', { fontSize: 14 }),
  node('plain-14px-wrap', 'The quick brown fox jumps over the lazy dog', 'text', { fontSize: 14 }),
  node('string-quoted', 'quoted string, with punctuation: a; b.', 'string'),
  node(
    'string-wrap',
    'Long quoted string that will wrap across several lines at width 120 pixels',
    'string'
  ),

  // Markdown labels: styles, breaks, dropped constructs, entities, block tokens.
  node('md-bold', '**bold** text', 'markdown'),
  node('md-italic', '*italic* text', 'markdown'),
  node('md-both', '***both*** text', 'markdown'),
  node('md-nested', '**a *b* c**', 'markdown'),
  node('md-underscore', '__u__ _e_', 'markdown'),
  node('md-mixed', 'This is **bold** and *italic* and ***both***', 'markdown'),
  node('md-soft-break', 'a\nb', 'markdown'),
  node('md-br-strong', 'x<br/>**y**', 'markdown'),
  node('md-blank-lines', 'line one<br/><br/>line three', 'markdown'),
  node('md-hard-break', 'a  \nb', 'markdown'),
  node('md-blank-line', 'a\n\nb', 'markdown'),
  node('md-codespan', '`code` and text', 'markdown'),
  node('md-link', '[link](https://example.com) here', 'markdown'),
  node('md-strikethrough', 'x ~~del~~ y', 'markdown'),
  node('md-escape', 'a \\*escaped\\* star', 'markdown'),
  node('md-image', 'x ![alt](i.png) y', 'markdown'),
  node('md-entities', '&amp; &lt;c&gt;', 'markdown'),
  node('md-entities-other', '&copy; &#35; &nbsp;x', 'markdown'),
  node('md-apostrophe', "it's", 'markdown'),
  node('md-quotes', 'She said "hi"', 'markdown'),
  node('md-brackets', 'a < b > c', 'markdown'),
  node('md-inline-html', '<span>styled</span> text', 'markdown'),
  node('md-heading', '# Heading text', 'markdown'),
  node('md-list', '- item one\n- item two', 'markdown'),
  node(
    'md-long-120',
    '**Bold words** and *italic words* make a long label that must wrap across several lines at width 120',
    'markdown'
  ),
  node('md-long-word-120', 'Pneumonoultramicroscopicsilicovolcanoconiosis **bold**', 'markdown'),
  node('md-indented', 'Start\n    indented', 'markdown'),
  node('md-backslash', 'C:\\path\\to', 'markdown'),
  node('md-empty', '', 'markdown'),
  node('md-14px', '**Start** with 14px text', 'markdown', { fontSize: 14 }),

  // Edge labels: the default width of 200 and the background rectangle.
  edge(
    'edge-plain-200',
    'The quick brown fox jumps over the lazy dog and keeps running far away',
    'text'
  ),
  edge('edge-plain-empty', '', 'text'),
  edge('edge-plain-14px', 'Edge label', 'text', { fontSize: 14 }),
  edge('edge-md-200', '**Edge** label with *em* and more words to wrap around', 'markdown'),
  edge('edge-md-14px', 'Edge *label* at 14', 'markdown', { fontSize: 14 }),

  // Styles: a font size sets the box and the wrap; weight and style are set on the text element.
  node('style-size-20', 'Start label', 'text', { style: 'font-size:20px;' }),
  node('style-size-20-long', 'The quick brown fox jumps over the lazy dog', 'text', {
    style: 'font-size:20px;',
  }),
  node('style-size-20-md', '**Start** with *more* text here', 'markdown', {
    style: 'font-size:20px;',
  }),
  node('style-bold', 'Bold style label words here', 'text', { style: 'font-weight:bold;' }),
  node('style-bold-md', 'Plain words and *em*', 'markdown', { style: 'font-weight:bold;' }),
  node('style-italic', 'Italic style label', 'text', { style: 'font-style:italic;' }),
  edge('style-edge-size-20', 'Edge label at twenty', 'text', { style: 'font-size:20px;' }),
  edge('style-edge-bold', 'Edge bold label', 'text', { style: 'font-weight:bold;' }),

  // Cluster titles: anchored at the start, so each line starts at x = 0. A plain title never wraps
  // (`createLabel` passes `Infinity`); a Markdown title wraps at the node width, 200 here. Its group
  // carries no style, so the wrap does not see the title's font size.
  node('cluster-plain-two-lines', 'Subgraph title<br/>second line', 'text', {
    anchor: 'start',
    width: NO_WRAP,
  }),
  node('cluster-plain-empty-middle', 'first<br/><br/>third', 'text', {
    anchor: 'start',
    width: NO_WRAP,
  }),
  node('cluster-plain-leading-empty', '<br/>after a blank line', 'text', {
    anchor: 'start',
    width: NO_WRAP,
  }),
  node('cluster-plain-trailing-empty', 'before<br/>', 'text', { anchor: 'start', width: NO_WRAP }),
  node('cluster-plain-entities', 'a &amp; b<br/>c &lt; d', 'text', {
    anchor: 'start',
    width: NO_WRAP,
  }),
  node('cluster-plain-14px', 'Title line one<br/>line two', 'text', {
    anchor: 'start',
    width: NO_WRAP,
    fontSize: 14,
  }),
  node('cluster-plain-style', 'Title at twenty<br/>second line', 'text', {
    anchor: 'start',
    width: NO_WRAP,
    style: 'font-size:20px;',
  }),
  node('cluster-plain-empty', '', 'text', { anchor: 'start', width: NO_WRAP }),
  node(
    'cluster-md-wrap',
    '**Bold** cluster title with *emphasis* that is long enough to wrap at two hundred pixels',
    'markdown',
    { anchor: 'start', width: 200 }
  ),
  node('cluster-md-two-lines', '**Start** line<br/>*second* line', 'markdown', {
    anchor: 'start',
    width: 200,
  }),
  node(
    'cluster-md-style',
    'Cluster title in markdown with *styled* text that is long enough to wrap around',
    'markdown',
    { anchor: 'start', width: 200, style: 'font-size:20px;' }
  ),

  // Ink past the advance: a glyph whose outline reaches past its advance widens the box `getBBox`
  // gives the text. The leading `j` reaches left of the origin, the trailing `q` and the italic `f`
  // and capitals reach past the end of their advance (`extents` in `chromium.json`).
  node('ink-leading-j', 'jump over the fence', 'text'),
  node('ink-leading-j-alone', 'j', 'text'),
  node('ink-trailing-q', 'a q', 'text'),
  node('ink-trailing-q-14', 'the quiz q', 'text', { fontSize: 14 }),
  node('ink-leading-j-14', 'jun', 'text', { fontSize: 14 }),
  node('ink-md-italic-f', 'an *of*', 'markdown'),
  node('ink-md-italic-j', '*jam* jelly', 'markdown'),
  node('ink-md-italic-capitals', '*Jj* and *Yj*', 'markdown'),
  node('ink-md-bold-j', '**jun** and more', 'markdown'),
  node('ink-wrap-120', 'a long line that wraps and ends with the trailing q of the quiz', 'text'),
  edge('ink-edge-j', 'just in time', 'text'),
  edge('ink-edge-md-italic-f', 'an *of* edge', 'markdown'),
  node('ink-cluster-start', 'Jq and fjord<br/>quit', 'text', {
    anchor: 'start',
    width: NO_WRAP,
  }),
  node('ink-cluster-md', '*Jj* title<br/>*fjord*', 'markdown', {
    anchor: 'start',
    width: NO_WRAP,
  }),
];

/** The fonts of the page: Lexica Ultralegible in its four styles, as data URLs. */
const FACE_FILES: [number, string, string][] = [
  [400, 'normal', 'LexicaUltralegible-Regular.ttf'],
  [400, 'italic', 'LexicaUltralegible-Italic.ttf'],
  [700, 'normal', 'LexicaUltralegible-Bold.ttf'],
  [700, 'italic', 'LexicaUltralegible-BoldItalic.ttf'],
];

/**
 * The page's code: the reference's `createText` and the measurement of its result. The entry is
 * bundled with `createText` from its source, so `createText` is the reference's own code.
 */
const entry = `
import { select } from 'd3';
import { createText } from ${JSON.stringify(createTextPath)};

const SVG = 'http://www.w3.org/2000/svg';

// A label the reference cannot lay out is recorded with its error, not measured.
window.measureLabel = async (request) => {
  try {
    return await measure(request);
  } catch (error) {
    return { error: String(error && error.message ? error.message : error) };
  }
};

const measure = async (request) => {
  const svg = document.createElementNS(SVG, 'svg');
  svg.setAttribute('width', '1000');
  svg.setAttribute('height', '1000');
  svg.setAttribute('style', 'font-family: "Lexica Ultralegible"; font-size: ' + request.fontSize + 'px;');
  document.body.appendChild(svg);
  const container = document.createElementNS(SVG, 'g');
  svg.appendChild(container);
  const markdown = request.labelType === 'markdown';
  const width = request.width === null ? undefined : request.width;
  let returned;
  if (request.anchor === 'start') {
    // clusters.js: createLabel (or createText for Markdown) in a 'g.cluster-label', which has no style
    // attribute and no stylesheet rule, so the text has no text-anchor and starts at x = 0.
    const labelEl = select(container).append('g').attr('class', 'cluster-label');
    returned = await createText(labelEl, request.text, {
      useHtmlLabels: false,
      width,
      style: request.style,
      addSvgBackground: request.addBackground,
      markdown,
      isNode: true,
    });
  } else if (request.isNode) {
    // labelHelper: a 'g.label' with the node's label style, the text drawn into it.
    const shape = select(container).append('g').attr('class', 'node default');
    const labelEl = shape.insert('g').attr('class', 'label').attr('style', request.style);
    returned = await createText(labelEl, request.text, {
      useHtmlLabels: false,
      width,
      minWidth: 0,
      classes: '',
      style: request.style,
      addSvgBackground: request.addBackground,
      markdown,
      isNode: true,
    });
  } else {
    // insertEdgeLabel: the text is drawn into the edge label container, styled afterwards.
    returned = await createText(select(container), request.text, {
      style: request.style,
      useHtmlLabels: false,
      addSvgBackground: request.addBackground,
      isNode: false,
      markdown,
      width,
    });
  }
  const textEl = returned.tagName.toLowerCase() === 'text' ? returned : returned.querySelector(':scope > text');
  const lines = [];
  const lineTextLengths = [];
  let index = 0;
  for (const line of textEl.querySelectorAll(':scope > tspan.text-outer-tspan')) {
    lineTextLengths.push(line.getComputedTextLength());
    const words = [];
    for (const tspan of line.querySelectorAll(':scope > tspan.text-inner-tspan')) {
      const computed = getComputedStyle(tspan);
      const text = tspan.textContent ?? '';
      const start = textEl.getStartPositionOfChar(index);
      words.push({
        text,
        weight: Number(computed.fontWeight),
        style: computed.fontStyle,
        fontSize: parseFloat(computed.fontSize),
        x: start.x,
        y: start.y,
      });
      index += text.length;
    }
    lines.push(words);
  }
  const box = textEl.getBBox();
  const rect = returned.tagName.toLowerCase() === 'g' ? returned.querySelector(':scope > rect.background') : null;
  const background = rect
    ? {
        x: Number(rect.getAttribute('x')),
        y: Number(rect.getAttribute('y')),
        width: Number(rect.getAttribute('width')),
        height: Number(rect.getAttribute('height')),
      }
    : null;
  svg.remove();
  return {
    lines,
    lineTextLengths,
    bbox: { x: box.x, y: box.y, width: box.width, height: box.height },
    background,
  };
};
`;

/** Bundles the entry with the reference's `createText` and the modules it imports. */
async function bundle(): Promise<string> {
  const result = await build({
    stdin: {
      contents: entry,
      resolveDir: dirname(createTextPath),
      sourcefile: 'label-entry.js',
      loader: 'js',
    },
    bundle: true,
    format: 'iife',
    platform: 'browser',
    target: 'es2022',
    write: false,
    logLevel: 'silent',
    // The configuration schema is read the way the Mermaid build reads it.
    plugins: [jsonSchemaPlugin],
    // Build-time flags of the Mermaid bundle: the profiler is off in production.
    define: { injected: '{"version":"fixture","profiling":false}' },
  });
  return result.outputFiles[0].text;
}

async function measureAll(): Promise<{ chromiumVersion: string; labels: Measured[] }> {
  const fontFaces = await Promise.all(
    FACE_FILES.map(async ([weight, style, file]) => {
      const data = (await readFile(join(fontDir, file))).toString('base64');
      return `@font-face { font-family: "Lexica Ultralegible"; font-weight: ${weight}; font-style: ${style}; src: url(data:font/ttf;base64,${data}) format("truetype"); }`;
    })
  );
  // The flowchart stylesheet rule that centres node labels.
  const styles = `${fontFaces.join('\n')}\n.node .label text { text-anchor: middle; }\nbody { margin: 0; }`;

  const browser = await chromium.launch({
    // Without hinting Chromium keeps fractional advances, as it does on macOS.
    args: ['--font-render-hinting=none'],
  });
  try {
    const page = await browser.newPage();
    await page.setContent(
      `<!doctype html><html><head><meta charset="utf-8"><style>${styles}</style></head><body></body></html>`
    );
    await page.evaluate(async () => {
      for (const font of ['normal 16px', 'italic 16px', 'bold 16px', 'italic bold 16px']) {
        await document.fonts.load(`${font} "Lexica Ultralegible"`);
      }
      await document.fonts.ready;
    });
    await page.addScriptTag({ content: await bundle() });

    const labels: Measured[] = [];
    for (const request of requests) {
      const measured = await page.evaluate(
        (input: Request) =>
          (window as unknown as { measureLabel: (r: Request) => Promise<unknown> }).measureLabel(
            input
          ),
        request
      );
      labels.push({ ...request, ...(measured as Omit<Measured, keyof Request>) });
    }
    return { chromiumVersion: browser.version(), labels };
  } finally {
    await browser.close();
  }
}

/** The differences between a fresh run and the committed fixture, as readable lines. */
function compare(fresh: Measured[], committed: Measured[]): string[] {
  const problems: string[] = [];
  if (fresh.length !== committed.length) {
    problems.push(`label count ${fresh.length} vs ${committed.length}`);
  }
  const near = (a: number, b: number, where: string) => {
    if (Math.abs(a - b) > TOLERANCE) problems.push(`${where}: ${a} vs ${b}`);
  };
  fresh.forEach((label, index) => {
    const was = committed[index];
    if (!was || was.id !== label.id) {
      problems.push(`${label.id}: label order changed`);
      return;
    }
    if (label.error !== was.error) {
      problems.push(`${label.id}: error ${label.error} vs ${was.error}`);
      return;
    }
    if (label.error) return;
    if (label.lines.length !== was.lines.length) {
      problems.push(`${label.id}: ${label.lines.length} lines vs ${was.lines.length}`);
      return;
    }
    label.lines.forEach((line, row) => {
      if (line.length !== was.lines[row].length) {
        problems.push(`${label.id} line ${row}: ${line.length} words vs ${was.lines[row].length}`);
        return;
      }
      line.forEach((word, column) => {
        const other = was.lines[row][column];
        if (
          word.text !== other.text ||
          word.weight !== other.weight ||
          word.style !== other.style
        ) {
          problems.push(`${label.id} line ${row} word ${column}: ${word.text} vs ${other.text}`);
        }
        near(word.x, other.x, `${label.id} line ${row} word ${column} x`);
        near(word.y, other.y, `${label.id} line ${row} word ${column} y`);
      });
    });
    label.lineTextLengths.forEach((length, row) =>
      near(length, was.lineTextLengths[row], `${label.id} line ${row} width`)
    );
    for (const key of ['x', 'y', 'width', 'height'] as const) {
      near(label.bbox[key], was.bbox[key], `${label.id} box ${key}`);
      if (label.background || was.background) {
        near(
          label.background?.[key] ?? NaN,
          was.background?.[key] ?? NaN,
          `${label.id} background ${key}`
        );
      }
    }
  });
  return problems;
}

async function main(): Promise<void> {
  const check = process.argv.includes('--check');
  const fresh = await measureAll();
  if (check) {
    const committed = JSON.parse(await readFile(fixturePath, 'utf8')) as { labels: Measured[] };
    const problems = compare(fresh.labels, committed.labels);
    if (problems.length > 0) {
      console.error(problems.join('\n'));
      console.error(`${problems.length} differences from ${fixturePath}`);
      process.exit(1);
    }
    console.log(`label fixture: ${fresh.labels.length} labels agree with the committed values`);
    return;
  }
  const output = {
    description:
      'Reference createText (htmlLabels: false) run in Chromium; see tools/port/text/label-fixture.ts',
    chromium: fresh.chromiumVersion,
    labels: fresh.labels,
  };
  await writeFile(fixturePath, `${JSON.stringify(output, null, 2)}\n`);
  console.log(`wrote ${fixturePath}: ${fresh.labels.length} labels`);
}

await main();
