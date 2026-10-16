// The addon in Node.js. The test builds the library, then loads it with `process.dlopen`, which is how
// Node loads a `.node` file.
//
// Run from the repository root: `node --test crates/ferrigura-node/test/render.test.mjs`
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../../', import.meta.url));
execFileSync('cargo', ['build', '-q', '-p', 'ferrigura-node'], { cwd: root, stdio: 'inherit' });

const library = {
  darwin: 'libferrigura_node.dylib',
  win32: 'ferrigura_node.dll',
}[process.platform] ?? 'libferrigura_node.so';
const addon = { exports: {} };
process.dlopen(addon, join(root, 'target/debug', library));
const { render } = addon.exports;

const PIE = 'pie title Pets\n  "Dogs" : 386\n  "Cats" : 85\n';

test('renders a pie to an SVG document with a derived id', () => {
  const svg = render(PIE);
  assert.match(svg, /^<svg /);
  assert.match(svg, /<\/svg>$/);
  assert.match(svg, /id="ferrigura-[0-9a-f]{16}"/);
});

test('the same input gives the same SVG', () => {
  assert.equal(render(PIE), render(PIE));
});

test('the id option names the diagram and scopes its styles', () => {
  const svg = render(PIE, undefined, { id: 'pie-one' });
  assert.match(svg, /id="pie-one"/);
  assert.match(svg, /#pie-one \.pieCircle/);
});

test('the auto colour scheme writes the dark set under the media query', () => {
  const svg = render(PIE, undefined, { id: 'pie-one', colorScheme: 'auto' });
  assert.match(svg, /@media \(prefers-color-scheme: dark\)\{#pie-one\{/);
});

test('the configuration applies its theme variables', () => {
  const svg = render(PIE, { themeVariables: { pie1: '#00ff00' } }, { colorScheme: 'light' });
  assert.match(svg, /--pie1: #00ff00;/);
});

test('a colour scheme that is not known is refused', () => {
  assert.throws(() => render(PIE, undefined, { colorScheme: 'blue' }), /unknown colour scheme blue/);
});

test('text that is not a diagram throws', () => {
  assert.throws(() => render('not a diagram\n'), /no diagram type detected/);
});
