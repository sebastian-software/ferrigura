/**
 * Shape probe: the sizes and outlines of the flowchart shapes of stage A, measured in Chromium on
 * the JavaScript reference, for both looks. Every case is a single-node flowchart; the node's
 * group, its outline element and its label are measured with `getBBox`, in their own user space.
 *
 * The result is stored as a fixture that the Rust test in
 * `crates/ferrigura-render/tests/flowchart_shapes.rs` feeds back into the port: each label box goes
 * to `draw`, and the node box, the label offset and the outline box are compared with what the
 * reference measured. Text is isolated: the reference runs with `htmlLabels: false` and Lexica
 * Ultralegible as a web font, so the label boxes are the reference's own, and only the shapes are
 * compared.
 *
 * Chromium is run with `--font-render-hinting=none`, as the other geometry harnesses are.
 *
 * Usage: `pnpm exec tsx tools/port/geometry/shape-probe.ts`
 */
import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { extname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { chromium, type Page } from 'playwright';

import { referenceRoot } from '../reference-root.ts';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const distDir = join(referenceRoot, 'packages/mermaid/dist');
const fontDir = join(root, 'crates/ferrigura-text/fonts');
const fixtureDir = join(root, 'crates/ferrigura-render/tests/fixtures');
/** The fixture of the stage A shapes, and of the stage C shapes of T3c (`--t3c`). */
const STAGE_A_FIXTURE = join(fixtureDir, 'flowchart-shapes.json');
const T3C_FIXTURE = join(fixtureDir, 'flowchart-shapes-t3c.json');

/**
 * The shapes of stages A and C, by the name the flowchart's `@{ shape: … }` directive takes. The
 * directive takes lowercase names without underscores (`flowDb.ts`), so a shape whose handler name
 * is camelCase is named by its short name or an alias.
 */
const SHAPES: [directive: string, internal: string][] = [
  ['rect', 'squareRect'],
  ['rounded', 'roundedRect'],
  ['stadium', 'stadium'],
  ['fr-rect', 'subroutine'],
  ['cyl', 'cylinder'],
  ['circle', 'circle'],
  ['diam', 'question'],
  ['hex', 'hexagon'],
  ['lean-r', 'lean_right'],
  ['lean-l', 'lean_left'],
  ['trap-b', 'trapezoid'],
  ['trap-t', 'inv_trapezoid'],
  ['dbl-circ', 'doublecircle'],
  ['text', 'text'],
  ['f-circ', 'filledCircle'],
  ['fork', 'forkJoin'],
  ['sm-circ', 'stateStart'],
  ['fr-circ', 'stateEnd'],
  ['anchor', 'anchor'],
  ['odd', 'rect_left_inv_arrow'],
  ['collapsed', 'collapsedGroup'],
  ['bang', 'bang'],
  ['bow-rect', 'bowTieRect'],
  ['browser', 'browser'],
  ['bucket', 'bucket'],
  ['notch-rect', 'card'],
  ['cloud', 'cloud'],
  ['console', 'consoleWindow'],
  ['cross-circ', 'crossedCircle'],
  ['brace', 'curlyBraceLeft'],
  ['brace-r', 'curlyBraceRight'],
  ['braces', 'curlyBraces'],
  ['curv-trap', 'curvedTrapezoid'],
  ['datastore', 'datastore'],
  ['div-rect', 'dividedRectangle'],
  ['flip-tri', 'flippedTriangle'],
  ['folder', 'folder'],
  ['delay', 'halfRoundedRectangle'],
  ['hourglass', 'hourglass'],
  ['choice', 'choice'],
  ['note', 'note'],
];

/**
 * The stage C shapes of this port (T3c): the shapes a flowchart reaches that stage A does not
 * have, the icon and image shapes included. A case records the node's `shape` as the flowchart's
 * data gives it, and the directive that is written for it. `props` are the directive's other
 * properties. `expectError` marks a case the reference cannot render: its error is recorded.
 */
interface Variant {
  shape: string;
  directive: string;
  internal?: string;
  props?: Record<string, string | number>;
  expectError?: boolean;
}

/** A 1 by 1 PNG, so the reference can decode the image of an image shape without a network. */
const PNG_DATA_URL =
  'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==';

const T3C_SHAPES: Variant[] = [
  { shape: 'person', directive: 'person' },
  { shape: 'bolt', directive: 'bolt' },
  { shape: 'lin-cyl', directive: 'lin-cyl' },
  { shape: 'lin-doc', directive: 'lin-doc' },
  { shape: 'st-rect', directive: 'st-rect' },
  { shape: 'docs', directive: 'docs' },
  { shape: 'lin-rect', directive: 'lin-rect' },
  { shape: 'sl-rect', directive: 'sl-rect' },
  { shape: 'tag-rect', directive: 'tag-rect' },
  { shape: 'tag-doc', directive: 'tag-doc' },
  { shape: 'h-cyl', directive: 'h-cyl' },
  { shape: 'notch-pent', directive: 'notch-pent' },
  { shape: 'tri', directive: 'tri' },
  { shape: 'doc', directive: 'doc' },
  { shape: 'flag', directive: 'flag' },
  { shape: 'win-pane', directive: 'win-pane' },
  // Icons: flowDb picks the shape from the icon and the form (`getTypeFromVertex`).
  { shape: 'icon', directive: 'icon' },
  { shape: 'icon', directive: 'icon', props: { icon: 'fa:bell' } },
  { shape: 'icon', directive: 'icon', props: { icon: 'fa:bell', pos: 't' } },
  { shape: 'iconSquare', directive: 'icon', props: { icon: 'fa:bell', form: 'square' } },
  {
    shape: 'iconSquare',
    directive: 'icon',
    props: { icon: 'fa:bell', form: 'square', pos: 't' },
  },
  { shape: 'iconRounded', directive: 'icon', props: { icon: 'fa:bell', form: 'rounded' } },
  { shape: 'iconCircle', directive: 'icon', props: { icon: 'fa:bell', form: 'circle' } },
  {
    shape: 'iconCircle',
    directive: 'icon',
    props: { icon: 'fa:bell', form: 'circle', pos: 't' },
  },
  { shape: 'imageSquare', directive: 'icon', props: { img: PNG_DATA_URL, w: 48, h: 48 } },
  { shape: 'imageSquare', directive: 'icon', props: { img: PNG_DATA_URL, w: 40, h: 60, pos: 't' } },
  {
    shape: 'imageSquare',
    directive: 'icon',
    props: { img: 'https://example.invalid/missing.png', w: 48, h: 48 },
    expectError: true,
  },
];

/** The labels: empty, short, a phrase, a long label that wraps, and two lines. */
const LABELS = [
  '',
  'A',
  'Process step',
  'A long label that needs wrapping across several lines of text',
  'two<br/>lines',
];

const LOOKS = ['neo', 'classic'] as const;

/** The site configuration of the reference, as `initialize` takes it (htmlLabels off). */
const referenceConfig = {
  htmlLabels: false,
  theme: 'redux-color',
  fontFamily: '"Lexica Ultralegible"',
  securityLevel: 'strict',
};

/** The harness page: loads the bundle, renders one flowchart and measures its node. */
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
window.measureNode = async (svgText, collapsed) => {
  const stage = document.getElementById('stage');
  stage.innerHTML = svgText;
  // The text is measured once the face is loaded and the page has laid it out: a label measured
  // earlier can read a few hundredths narrower, in the fallback face.
  await document.fonts.load('14px "Lexica Ultralegible"');
  await document.fonts.ready;
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  const svg = stage.querySelector('svg');
  const node = collapsed
    ? [...svg.querySelectorAll('g.node')].find((element) =>
        element.querySelector(':scope > rect.collapsed-group')
      )
    : // An icon or an image has the class of its labelHelper (icon-shape default), not node.
      [...svg.querySelectorAll('g.node'), ...svg.querySelectorAll('g')].find((element) =>
        /(^|-)flowchart-A-\\d+$/.test(element.id)
      );
  if (!node) throw new Error('no node A in the reference SVG');
  // The reference measures the labels while the diagram is still unscaled and the node is at the
  // origin: before the viewBox is set and before the layout moves the node. Chromium rounds glyph
  // positions to 1/64 px in the page, so a label measured at its laid-out position reads a few
  // hundredths narrower (79.114 instead of 79.172 for "Process step"), which the reference's
  // circles do not see. Measure in that arrangement, as the reference does at layout time.
  const laidOut = node.getAttribute('transform');
  node.setAttribute('transform', 'translate(0, 0)');
  svg.removeAttribute('viewBox');
  const box = (bbox) => ({ x: bbox.x, y: bbox.y, width: bbox.width, height: bbox.height });
  const nodeBox = node.getBBox();
  const attributes = (element) =>
    Object.fromEntries([...element.attributes].map((attribute) => [attribute.name, attribute.value]));
  const outline = node.firstElementChild;
  const label = [...node.children].find((child) => child.tagName === 'g' && child.classList.contains('label'));
  const labelText = label ? label.querySelector('text') : null;
  // The group that holds the label's background, when the label has one (icons and images): the
  // reference measures this group, not the text, for the node's label.
  const labelGroup = label ? label.querySelector(':scope > g') : null;
  return {
    id: node.id,
    className: node.getAttribute('class'),
    node: box(nodeBox),
    laidOutTransform: laidOut,
    childTags: [...node.children].map((child) => child.tagName),
    outline: outline
      ? { tag: outline.tagName, attributes: attributes(outline), bbox: box(outline.getBBox()) }
      : null,
    labelGroup: labelGroup ? box(labelGroup.getBBox()) : null,
    label: label
      ? {
          transform: label.getAttribute('transform'),
          attributes: attributes(label),
          text: labelText ? labelText.textContent : '',
          bbox: labelText ? box(labelText.getBBox()) : null,
        }
      : null,
  };
};
window.ready = true;
</script></body></html>`;

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

/**
 * The flowchart text of one case: a single node with the label and the variant's properties. A
 * collapsed subgraph is the `collapsedGroup` node: its title is the label, and it holds one vertex.
 */
function diagram(variant: Variant, label: string): string {
  if (variant.directive === 'collapsed') {
    return `flowchart TD\n    subgraph S["${label}"]\n      A\n    end\n    S@{ view: collapsed }\n`;
  }
  const props = Object.entries(variant.props ?? {})
    .map(([key, value]) => `, ${key}: ${typeof value === 'number' ? value : JSON.stringify(value)}`)
    .join('');
  return `flowchart TD\n    A@{ shape: ${variant.directive}, label: "${label}"${props} }\n`;
}

interface Measurement {
  id: string;
  labelGroup: { x: number; y: number; width: number; height: number } | null;
  className: string | null;
  node: { x: number; y: number; width: number; height: number };
  childTags: string[];
  outline: {
    tag: string;
    attributes: Record<string, string>;
    bbox: { x: number; y: number; width: number; height: number };
  } | null;
  label: {
    transform: string | null;
    attributes: Record<string, string>;
    text: string;
    bbox: { x: number; y: number; width: number; height: number } | null;
  } | null;
}

async function main(): Promise<void> {
  const t3c = process.argv.includes('--t3c');
  const variants: Variant[] = t3c
    ? T3C_SHAPES
    : SHAPES.map(([directive, internal]) => ({ shape: directive, directive, internal, props: {} }));
  const fixturePath = t3c ? T3C_FIXTURE : STAGE_A_FIXTURE;
  const { server, port } = await serve();
  const browser = await chromium.launch({
    // As in the pie geometry harness: fractional glyph advances, as on macOS.
    args: ['--font-render-hinting=none'],
  });
  const cases: unknown[] = [];
  const failures: string[] = [];
  const referenceErrors: string[] = [];
  try {
    const page: Page = await browser.newPage({ viewport: { width: 1200, height: 900 } });
    page.on('pageerror', (error) => console.error('page error:', error.message));
    await page.goto(`http://127.0.0.1:${port}/`);
    await page.waitForFunction(
      () => (window as unknown as { ready?: boolean }).ready === true,
      null,
      { timeout: 60_000 }
    );

    let counter = 0;
    for (const look of LOOKS) {
      for (const variant of variants) {
        for (const label of LABELS) {
          // The parser rejects an empty subgraph title, so a collapsed group is never empty.
          if (variant.directive === 'collapsed' && label === '') continue;
          counter += 1;
          const text = diagram(variant, label);
          const id = `probe-${counter}`;
          const name = `${variant.shape} ${look} "${label}"`;
          try {
            const svg = await page.evaluate(
              ([svgId, source, config]) =>
                (
                  window as unknown as { renderReference: (...a: unknown[]) => Promise<string> }
                ).renderReference(svgId, source, config),
              [id, text, { ...referenceConfig, look }] as const
            );
            const measurement = (await page.evaluate(
              ([markup, collapsed]) =>
                (
                  window as unknown as {
                    measureNode: (s: string, c: boolean) => Promise<unknown>;
                  }
                ).measureNode(markup, collapsed),
              [svg, variant.directive === 'collapsed'] as const
            )) as Measurement;
            cases.push({
              shape: variant.shape,
              ...(variant.internal ? { internal: variant.internal } : {}),
              props: variant.props ?? {},
              look,
              labelSource: label,
              ...measurement,
            });
          } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            if (variant.expectError) {
              referenceErrors.push(`${name}: ${message}`);
            } else {
              failures.push(`${name}: ${message}`);
            }
          }
        }
      }
    }
  } finally {
    await browser.close();
    server.close();
  }

  await mkdir(fixtureDir, { recursive: true });
  const fixture = {
    description: t3c
      ? 'Stage C flowchart shapes (T3c): the shapes, icons and images that stage A does not have, measured in Chromium on the JavaScript reference (htmlLabels false, redux-color, Lexica Ultralegible). Written by tools/port/geometry/shape-probe.ts --t3c; read by crates/ferrigura-render/tests/flowchart_shapes.rs.'
      : 'Flowchart shapes of stage A, measured in Chromium on the JavaScript reference (htmlLabels false, redux-color, Lexica Ultralegible). Written by tools/port/geometry/shape-probe.ts; read by crates/ferrigura-render/tests/flowchart_shapes.rs.',
    config: referenceConfig,
    referenceErrors,
    cases,
  };
  await writeFile(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);
  console.log(`shape probe: ${cases.length} cases written to ${fixturePath}`);
  for (const error of referenceErrors) console.log(`  reference error (expected): ${error}`);
  for (const failure of failures) console.log(`  not measured: ${failure}`);
  if (failures.length > 0) process.exit(1);
}

await main();
