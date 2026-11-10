// The addon in Node.js. The test builds the library, then loads it with `process.dlopen`, which is how
// Node loads a `.node` file.
//
// Run from the repository root: `node --test crates/ferrigura-node/test/render.test.mjs`
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { join } from 'node:path';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../../', import.meta.url));
execFileSync('cargo', ['build', '-q', '-p', 'ferrigura-node'], { cwd: root, stdio: 'inherit' });

const library =
  {
    darwin: 'libferrigura_node.dylib',
    win32: 'ferrigura_node.dll',
  }[process.platform] ?? 'libferrigura_node.so';
const addon = { exports: {} };
process.dlopen(addon, join(root, 'target/debug', library));
const { render, Renderer } = addon.exports;

const PIE = 'pie title Pets\n  "Dogs" : 386\n  "Cats" : 85\n';

void test('flowchart routing is adaptive by default and compatibility is per render', () => {
  const text = 'flowchart TD\n A[Draft] --> B{Review}\n B -->|Changes| A\n B --> C[Publish]\n';
  const renderer = new Renderer();
  const compatible = renderer.render(text, { compat: true });
  assert.equal(renderer.render(text), render(text, {}, { compat: false }));
  assert.notEqual(renderer.render(text), compatible);
  assert.equal(renderer.render(text, { compat: true }), compatible);
});

void test('renders a pie to an SVG document with a derived id', () => {
  const svg = render(PIE);
  assert.match(svg, /^<svg /);
  assert.match(svg, /<\/svg>$/);
  assert.match(svg, /id="ferrigura-[\da-f]{16}"/);
});

void test('the same input gives the same SVG', () => {
  assert.equal(render(PIE), render(PIE));
});

void test('the id option names the diagram and scopes its styles', () => {
  const svg = render(PIE, undefined, { id: 'pie-one' });
  assert.match(svg, /id="pie-one"/);
  assert.match(svg, /#pie-one \.pieCircle/);
});

void test('the auto colour scheme writes light-dark() values', () => {
  const svg = render(PIE, undefined, { id: 'pie-one', colorScheme: 'auto' });
  assert.match(svg, /svg:root{color-scheme:light dark;}#pie-one{[^}]*light-dark\(/);
  assert.doesNotMatch(svg, /prefers-color-scheme/);
});

void test('the configuration applies its theme variables', () => {
  const svg = render(PIE, { themeVariables: { pie1: '#00ff00' } }, { colorScheme: 'light' });
  assert.match(svg, /--pie1: #00ff00;/);
});

void test('a colour scheme that is not known is refused', () => {
  assert.throws(
    () => render(PIE, undefined, { colorScheme: 'blue' }),
    /unknown colour scheme blue/
  );
});

void test('text that is not a diagram throws', () => {
  assert.throws(() => render('not a diagram\n'), /no diagram type detected/);
});

const DIRECTED = '%%{init: {"theme": "forest"}}%%\n' + PIE;

void test('a renderer gives the SVG of render with its configuration', () => {
  const config = { themeVariables: { pie1: '#00ff00' } };
  const renderer = new Renderer(config);
  assert.equal(
    renderer.render(PIE, { colorScheme: 'auto' }),
    render(PIE, config, { colorScheme: 'auto' })
  );
  assert.equal(renderer.render(PIE), render(PIE, config));
});

void test('a renderer without a configuration gives the defaults', () => {
  assert.equal(new Renderer().render(PIE), render(PIE, {}));
});

void test('a renderer serves many renders, and directives merge on top of its configuration', () => {
  const renderer = new Renderer({ themeVariables: { pie1: '#00ff00' } });
  const first = renderer.render(DIRECTED);
  assert.equal(renderer.render(PIE), render(PIE, { themeVariables: { pie1: '#00ff00' } }));
  assert.equal(first, renderer.render(DIRECTED));
  assert.equal(first, render(DIRECTED, { themeVariables: { pie1: '#00ff00' } }));
});

void test('a renderer refuses a colour scheme that is not known, and text that is not a diagram', () => {
  const renderer = new Renderer();
  assert.throws(() => renderer.render(PIE, { colorScheme: 'blue' }), /unknown colour scheme blue/);
  assert.throws(() => renderer.render('not a diagram\n'), /no diagram type detected/);
});

void test('compat selects Mermaid geometry for flowcharts and sequence participants', () => {
  const renderer = new Renderer();
  const sequence = 'sequenceDiagram\n participant App\n participant API\n App->>API: Request\n';
  const compact = renderer.render(sequence);
  const compatible = renderer.render(sequence, { compat: true });
  assert.match(compact, /height="32" name="App"/);
  assert.match(compatible, /height="76" name="App"/);
  assert.equal(renderer.render(sequence, { compat: false }), compact);
  assert.equal(renderer.render(sequence, { compat: true }), compatible);
  const flow = 'flowchart TD\n A[Draft] --> B{Review}\n B -->|Changes| A\n B --> C[Publish]\n';
  assert.notEqual(renderer.render(flow), renderer.render(flow, { compat: true }));
});

const FONT_DATA = readFileSync(
  join(root, 'crates/ferrigura-text/fonts/AtkinsonHyperlegibleMono-Regular.ttf')
);
const FONT_TEXT = 'flowchart LR\n A[Wide WWW narrow iii]\n';
void test('host byte arrays register owned faces and default to weight 400 normal', () => {
  const bytes = new Uint8Array(FONT_DATA);
  const fonts = { fonts: [{ family: 'Project Mono', data: bytes }] };
  const renderer = new Renderer(undefined, fonts);
  const custom = renderer.render(FONT_TEXT, { id: 'fonts' });
  assert.notEqual(custom, render(FONT_TEXT, undefined, { id: 'fonts' }));
  assert.notEqual(
    custom.match(/viewBox="([^"]+)"/)[1],
    render(FONT_TEXT, undefined, { id: 'fonts' }).match(/viewBox="([^"]+)"/)[1]
  );
  assert.notEqual(
    custom.match(/data:font\/[^;]+;base64,([^"')]+)/)[1],
    render(FONT_TEXT, undefined, { id: 'fonts' }).match(/data:font\/[^;]+;base64,([^"')]+)/)[1]
  );
  assert.match(custom, /data:font\//);
  assert.equal(custom, render(FONT_TEXT, undefined, { id: 'fonts' }, fonts));
  assert.equal(
    custom,
    render(
      FONT_TEXT,
      undefined,
      { id: 'fonts' },
      { fonts: [{ family: 'Project Mono', data: FONT_DATA, weight: 400, style: 'normal' }] }
    )
  );
  bytes.fill(0);
  assert.equal(custom, renderer.render(FONT_TEXT, { id: 'fonts' }));
  assert.match(
    new Renderer(undefined, {
      fonts: [{ family: 'Numeric Weight', data: FONT_DATA, weight: 600, style: 'italic' }],
    }).render(FONT_TEXT),
    /data:font\//
  );
});
void test('font registration errors are eager and numeric metadata stays bounded', () => {
  assert.throws(
    () => new Renderer(undefined, { fonts: [{ family: 'Broken', data: Buffer.from('invalid') }] }),
    /font/i
  );
  for (const weight of [1, 600, 1000]) {
    assert.doesNotThrow(
      () => new Renderer(undefined, { fonts: [{ family: 'Weight', data: FONT_DATA, weight }] })
    );
  }
  for (const weight of [0, 1001, 2.5, NaN, Infinity]) {
    assert.throws(
      () => new Renderer(undefined, { fonts: [{ family: 'Weight', data: FONT_DATA, weight }] }),
      /font weight/
    );
  }
  assert.throws(
    () =>
      render(FONT_TEXT, undefined, undefined, {
        fonts: [{ family: 'Style', data: FONT_DATA, style: 'oblique' }],
      }),
    /font style/
  );
});
