/* eslint-disable security/detect-non-literal-fs-filename -- Paths are a fresh temporary directory and fixed files in this repository. */
// Renders the diagrams of the side-by-side page twice, with the published
// @ferrigura/core and with the published mermaid package in headless Chromium,
// and writes app/data/comparison.json and the SVG files in
// app/data/comparison/. The page imports them; nobody edits them by hand.
//
//   node scripts/render-comparison.mjs          # write the files
//   node scripts/render-comparison.mjs --check  # fail when they drifted
//
// Writing renders with the Ferrigura release the homepage's drawings come
// from (app/data/diagram-samples.json, written by render-diagram-samples.mjs)
// and the Mermaid release the port is checked against (tools/reference/pin),
// and records both in `renderedWith`; the check renders again with the
// recorded versions. Run the write again after `pnpm sample:write` or a move
// of the pin.
//
// Both sides get the settings the geometry check gives them
// (tools/port/geometry/common.ts): Mermaid draws its labels as SVG text, in
// Ferrigura's bundled face, with the redux-color theme that Ferrigura ships;
// flowcharts are drawn in Ferrigura's Mermaid mode. The page shows Mermaid's
// SVG unchanged and declares the face for it.
import assert from 'node:assert/strict';
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { extname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { format } from 'oxfmt';
import { chromium } from 'playwright';

import { compareOutput, installPublished } from './published-output.mjs';

const check = process.argv.includes('--check');
const repository = new URL('../../', import.meta.url);
const target = new URL('../app/data/comparison.json', import.meta.url);
const svgDirectory = new URL('../app/data/comparison/', import.meta.url);
const fontDirectory = new URL('crates/ferrigura-text/fonts/', repository);
const recorded = check ? JSON.parse(await readFile(target, 'utf8')) : undefined;
// The release the homepage's drawings come from, so both pages show the same one.
const homepageSamples = JSON.parse(
  await readFile(new URL('../app/data/diagram-samples.json', import.meta.url), 'utf8')
);
const pin = await readFile(new URL('tools/reference/pin', repository), 'utf8');
const pinnedMermaid = /^release mermaid@(\S+)$/m.exec(pin)?.[1];
assert(pinnedMermaid, 'tools/reference/pin names no Mermaid release');
const versions = recorded?.renderedWith ?? {
  '@ferrigura/core': homepageSamples.renderedWith['@ferrigura/core'],
  mermaid: pinnedMermaid,
};
const platform = `${process.platform}-${process.arch}`;

/**
 * The configuration both sides get, as the geometry check gives it: SVG
 * labels, Ferrigura's bundled face and the redux-color theme Ferrigura ships.
 */
const diagramConfig = {
  theme: 'redux-color',
  htmlLabels: false,
  flowchart: { htmlLabels: false },
  fontFamily: '"Lexica Ultralegible"',
};
/** Mermaid's page setup on top: no automatic run, strict security, stable ids. */
const mermaidConfig = {
  ...diagramConfig,
  startOnLoad: false,
  securityLevel: 'strict',
  deterministicIds: true,
};
/** Ferrigura's render options: a light page, and Mermaid's flowchart routing. */
const ferriguraOptions = { colorScheme: 'light', flowchartMode: 'mermaid' };

/** The examples: everyday diagrams of medium size, one per type. */
const examples = [
  {
    id: 'flowchart',
    title: 'Flowchart with subgraphs',
    description:
      'A delivery pipeline: a push is built and tested, a green build goes to staging and, after its smoke tests, to production; a failure leads back to a fix.',
    source: `flowchart TD
  subgraph ci [Continuous integration]
    push[Push to main] --> build[Build] --> tests{Tests pass?}
  end
  subgraph cd [Delivery]
    staging[Deploy to staging] --> smoke{Smoke tests}
    smoke -->|green| production[Deploy to production]
  end
  tests -->|yes| staging
  tests -->|no| fix[Fix the build]
  smoke -->|red| rollback[Roll back]
  rollback -.-> fix
  fix --> push
`,
  },
  {
    id: 'sequence',
    title: 'Sequence with blocks and notes',
    description:
      'A user signs in through an identity provider; the app trades the code for a token and then calls the API with it on every page.',
    source: `sequenceDiagram
  autonumber
  actor User
  participant App
  participant Auth as Identity provider
  participant API
  User->>App: Open the dashboard
  App->>Auth: Redirect to sign-in
  activate Auth
  Auth-->>User: Sign-in form
  User->>Auth: Credentials
  alt Valid credentials
    Auth-->>App: Authorization code
  else Invalid credentials
    Auth-->>User: Error message
  end
  deactivate Auth
  App->>Auth: Exchange the code
  Auth-->>App: Access token
  loop Every page
    App->>API: Request with token
    API-->>App: JSON
  end
  Note over App,API: The token expires after an hour
`,
  },
  {
    id: 'class',
    title: 'Class diagram',
    description:
      'A library holds books by one or more authors; each book has copies, a copy can be lent, and a member borrows copies through loans.',
    source: `classDiagram
  direction LR
  class Library {
    +String name
    +add(Book book) void
    +search(String query) List~Book~
  }
  class Book {
    +String isbn
    +String title
    +int year
  }
  class Author {
    +String name
  }
  class Copy {
    +int number
    +isAvailable() bool
  }
  class Lendable {
    <<interface>>
    +checkOut() void
  }
  class Member {
    +String id
    +borrow(Copy copy) Loan
  }
  class Loan {
    +Date due
    +isOverdue() bool
  }
  Library "1" *-- "*" Book : holds
  Book "*" --> "1..*" Author : written by
  Book "1" *-- "*" Copy
  Copy ..|> Lendable
  Member "1" --> "*" Loan
  Loan --> Copy
`,
  },
  {
    id: 'state',
    title: 'State diagram with composite states',
    description:
      'An order moves from the cart through a checkout of three steps; a choice on the payment sends it to fulfillment or back to the cart.',
    source: `stateDiagram-v2
  [*] --> Cart
  Cart --> Checkout: check out
  state Checkout {
    [*] --> Address
    Address --> Payment
    Payment --> Confirmation
    Confirmation --> [*]
  }
  state paid <<choice>>
  Checkout --> paid
  paid --> Fulfillment: accepted
  paid --> Cart: declined
  state Fulfillment {
    [*] --> Picking
    Picking --> Packed
    Packed --> Shipped
    Shipped --> [*]
  }
  Fulfillment --> Delivered
  Delivered --> [*]
  note right of Cart: Items can be added at any time
`,
  },
  {
    id: 'er',
    title: 'Entity relationship',
    description:
      'A shop schema: customers place orders of line items, each line item names a product, products belong to categories and customers keep addresses.',
    source: `erDiagram
  CUSTOMER ||--o{ ORDER : places
  ORDER ||--|{ LINE_ITEM : contains
  PRODUCT ||--o{ LINE_ITEM : "ordered as"
  CATEGORY ||--o{ PRODUCT : groups
  CUSTOMER ||--o{ ADDRESS : "ships to"
  CUSTOMER {
    int id PK
    string name
    string email UK
  }
  ORDER {
    int id PK
    int customer_id FK
    date placed
    string status
  }
  LINE_ITEM {
    int order_id FK
    int product_id FK
    int quantity
  }
  PRODUCT {
    int id PK
    string title
    decimal price
  }
`,
  },
  {
    id: 'gantt',
    title: 'Gantt chart',
    description:
      'A release plan in three sections: design, then the engine, the command line and its documentation in parallel, then a beta and the release.',
    source: `gantt
  title Release 2.0
  dateFormat YYYY-MM-DD
  axisFormat %b %d
  todayMarker off
  section Design
    Requirements   :done, req, 2026-01-05, 5d
    Architecture   :done, arch, after req, 4d
  section Build
    Core engine    :active, core, after arch, 10d
    Command line   :cli, after arch, 6d
    Documentation  :docs, after cli, 5d
  section Ship
    Beta           :milestone, beta, after core, 0d
    Release        :crit, release, after docs core, 3d
`,
  },
  {
    id: 'gitgraph',
    title: 'Git graph',
    description:
      'A feature branch and a hotfix leave main; the hotfix is merged and tagged, main is merged into the feature, and the feature is merged and tagged.',
    source: `gitGraph
  commit id: "init"
  commit id: "readme"
  branch feature
  checkout feature
  commit id: "parser"
  commit id: "tests"
  checkout main
  branch hotfix
  checkout hotfix
  commit id: "fix crash"
  checkout main
  merge hotfix tag: "v1.0.1"
  checkout feature
  merge main
  commit id: "renderer"
  checkout main
  merge feature tag: "v1.1.0"
`,
  },
  {
    id: 'mindmap',
    title: 'Mindmap',
    description:
      'A trip plan around one root: travel, stay, the things to see and the things to pack, each with its own branches.',
    source: `mindmap
  root((Trip to Lisbon))
    Travel
      Flight
      Airport shuttle
    Stay
      Hotel in Alfama
      Late check-out
    See
      Tram 28
      Belém
        Tower
        Monastery
      Sintra day trip
    Pack
      Sunscreen
      Walking shoes
`,
  },
  {
    id: 'pie',
    title: 'Pie chart',
    description: 'Where the hours of a sprint went, with the values shown in the legend.',
    source: `pie showData
  title Hours in a sprint
  "Features" : 42
  "Bug fixes" : 18
  "Reviews" : 14
  "Meetings" : 11
  "Documentation" : 7
`,
  },
  {
    id: 'xychart',
    title: 'XY chart',
    description: 'Monthly downloads over half a year, as bars with a line for the target on top.',
    source: `xychart-beta
  title "Monthly downloads"
  x-axis [Jan, Feb, Mar, Apr, May, Jun]
  y-axis "Downloads (thousands)" 0 --> 50
  bar [12, 17, 21, 30, 34, 45]
  line [10, 15, 20, 25, 30, 35]
`,
  },
];

const packages = await installPublished(versions, { quiet: check });
let svgs;
try {
  svgs = await renderAll(packages);
} finally {
  await packages.remove();
}

const data = {
  renderedWith: versions,
  platform: check ? recorded.platform : platform,
  settings: { config: diagramConfig, mermaid: mermaidConfig, ferrigura: ferriguraOptions },
  examples: examples.map((example) => ({
    ...example,
    ferrigura: `ferrigura-${example.id}.svg`,
    mermaid: `mermaid-${example.id}.svg`,
  })),
};
const formatted = await format('comparison.json', JSON.stringify(data));
assert.equal(formatted.errors.length, 0);
const drawnWith = `@ferrigura/core ${versions['@ferrigura/core']} and mermaid ${versions.mermaid}`;

if (check) {
  const exact = recorded.platform === platform;
  const message = `The side-by-side drawings drifted from ${drawnWith}; run pnpm comparison:write`;
  const json = await readFile(target, 'utf8');
  compareOutput({
    actual: json,
    expected: formatted.code,
    label: 'comparison.json',
    exact,
    message,
  });
  const entries = await readdir(svgDirectory);
  const committed = entries.filter((entry) => entry.endsWith('.svg'));
  assert.deepEqual(committed.toSorted(), [...svgs.keys()].toSorted(), 'The SVG files drifted');
  for (const [entry, content] of svgs) {
    const actual = await readFile(new URL(entry, svgDirectory), 'utf8');
    compareOutput({ actual, expected: content, label: entry, exact, message });
  }
  const tolerance = exact ? '' : ` (${platform}, numbers within 0.01 of ${recorded.platform})`;
  console.log(
    `Verified ${examples.length} side-by-side diagrams against ${drawnWith}${tolerance}.`
  );
} else {
  await rm(svgDirectory, { recursive: true, force: true });
  await mkdir(svgDirectory, { recursive: true });
  for (const [entry, content] of svgs) await writeFile(new URL(entry, svgDirectory), content);
  await writeFile(target, formatted.code);
  console.log(`Wrote ${examples.length} side-by-side diagrams rendered by ${drawnWith}.`);
}

/** Every example from both packages, by file name. */
async function renderAll({ directory, require }) {
  const { Renderer } = await import(pathToFileURL(require.resolve('@ferrigura/core')).href);
  const renderer = new Renderer(diagramConfig);
  const mermaid = await renderMermaid(join(directory, 'node_modules', 'mermaid', 'dist'));
  const rendered = new Map();
  for (const example of examples) {
    rendered.set(`ferrigura-${example.id}.svg`, renderFerrigura(renderer, example));
    rendered.set(`mermaid-${example.id}.svg`, mermaid.get(example.id));
  }
  return rendered;
}

function renderFerrigura(renderer, example) {
  const options = { ...ferriguraOptions, id: `ferrigura-${example.id}` };
  const svg = renderer.render(example.source, options);
  assert.equal(
    svg,
    renderer.render(example.source, options),
    `${example.id} must render the same twice`
  );
  assert.doesNotMatch(svg, /<foreignObject\b|<script\b/, `${example.id} contains HTML or script`);
  return svg;
}

/** Mermaid's drawing of every example, each rendered twice on a fresh page. */
async function renderMermaid(mermaidDist) {
  const { page, close } = await openMermaidPage(mermaidDist);
  try {
    const rendered = new Map();
    for (const example of examples) {
      const id = `mermaid-${example.id}`;
      const first = await renderOnFreshPage(page, id, example.source);
      const second = await renderOnFreshPage(page, id, example.source);
      assert.equal(first, second, `Mermaid must render ${example.id} the same twice`);
      assert.doesNotMatch(
        first,
        /<foreignObject\b|<script\b/,
        `Mermaid's ${id} contains HTML or script`
      );
      rendered.set(example.id, first);
    }
    return rendered;
  } finally {
    await close();
  }
}

/** A fresh page for every render: Mermaid counts some ids across the renders of one page. */
async function renderOnFreshPage(page, id, source) {
  await page.reload();
  await page.waitForFunction(() => globalThis.ready === true, null, { timeout: 60_000 });
  const result = await page.evaluate(
    ([svgId, text]) => globalThis.renderDiagram(svgId, text),
    [id, source]
  );
  assert(result.svg, `Mermaid did not render ${id}: ${result.error}`);
  return result.svg;
}

/**
 * A Chromium page with Mermaid initialized with `mermaidConfig` and the
 * bundled faces loaded, under every family name Mermaid measures with, all
 * served from 127.0.0.1. As in the geometry check,
 * Chromium runs with `--font-render-hinting=none`, in UTC, at a 1200 px
 * viewport, and Math.random is seeded, so the mindmap's layout starts the
 * same way on every run.
 */
async function openMermaidPage(mermaidDist) {
  const routes = [
    ['/mermaid/', mermaidDist],
    ['/fonts/', fileURLToPath(fontDirectory)],
  ];
  const html = pageHtml();
  const server = createServer((request, response) => {
    const { pathname } = new URL(request.url ?? '/', 'http://127.0.0.1');
    if (pathname === '/') response.writeHead(200, { 'content-type': 'text/html' }).end(html);
    else void serveFile(routes, pathname, response);
  });
  await new Promise((resolve) => {
    server.listen(0, '127.0.0.1', resolve);
  });
  const browser = await chromium.launch({ args: ['--font-render-hinting=none'] });
  const page = await browser.newPage({
    viewport: { width: 1200, height: 900 },
    screen: { width: 1200, height: 900 },
    timezoneId: 'UTC',
  });
  page.on('pageerror', (error) => console.error('page error:', error.message));
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  const close = async () => {
    await browser.close();
    server.close();
  };
  return { page, close };
}

async function serveFile(routes, pathname, response) {
  const types = { '.mjs': 'text/javascript', '.js': 'text/javascript', '.ttf': 'font/ttf' };
  const route = routes.find(([prefix]) => pathname.startsWith(prefix));
  const name = route ? decodeURIComponent(pathname.slice(route[0].length)) : '..';
  try {
    if (name.includes('..')) throw new Error('Outside the served folders');
    const body = await readFile(join(route[1], name));
    const type = types[extname(name)] ?? 'application/octet-stream';
    response.writeHead(200, { 'content-type': type }).end(body);
  } catch {
    response.writeHead(404).end();
  }
}

/** The page: the faces, a seeded Math.random, and Mermaid behind `renderDiagram`. */
function pageHtml() {
  const faces = [
    ['Regular', 400, 'normal'],
    ['Bold', 700, 'normal'],
    ['Italic', 400, 'italic'],
    ['BoldItalic', 700, 'italic'],
  ];
  // Mermaid measures some text in the families it names itself, which each
  // machine resolves to its own fonts. As in the geometry check (MAPPED_FAMILIES
  // in tools/port/geometry/common.ts), they all name the bundled face here.
  const families = [
    'Lexica Ultralegible',
    'trebuchet ms',
    'verdana',
    'arial',
    'Helvetica Neue',
    'Helvetica',
    'Open Sans',
  ];
  const fontFaces = families
    .flatMap((family) =>
      faces.map(
        ([file, weight, style]) =>
          `@font-face { font-family: "${family}"; src: url(/fonts/LexicaUltralegible-${file}.ttf) format("truetype"); font-weight: ${weight}; font-style: ${style}; }`
      )
    )
    .join('\n');
  // Every face is loaded before Mermaid measures, or it measures a fallback.
  const loads = families.flatMap((family) =>
    faces.map(([, weight, style]) => `${style} ${weight} 16px "${family}"`)
  );
  return `<!doctype html>
<html><head><meta charset="utf-8">
<style>
${fontFaces}
body { margin: 0; background: #ffffff; }
</style>
<script>
let seed = 0x2f6b3a1d;
Math.random = () => {
  seed = (seed + 0x6d2b79f5) | 0;
  let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
  t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
};
</script>
</head><body>
<script type="module">
import mermaid from '/mermaid/mermaid.esm.min.mjs';
await Promise.all(${JSON.stringify(loads)}.map((face) => document.fonts.load(face)));
mermaid.initialize(${JSON.stringify(mermaidConfig)});
globalThis.renderDiagram = async (id, text) => {
  try {
    const { svg } = await mermaid.render(id, text);
    return { svg, error: null };
  } catch (error) {
    return { svg: null, error: String(error) };
  }
};
globalThis.ready = true;
</script>
</body></html>`;
}
