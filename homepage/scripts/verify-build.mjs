/* eslint-disable security/detect-non-literal-fs-filename -- Every page comes from the repository-owned route manifest. */
// Checks the prerendered site after `ardo build`: every page wears the
// Monografie header and footer, documentation keeps its reading navigation,
// the homepage carries the release, its real drawings with their credit and
// the figures of the coverage table, and the only local stylesheet stays
// scoped to the homepage's specimens.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { expectedPages } from './site-pages.mjs';

/** A selector list split at its top-level commas, so `:is(a, b)` stays whole. */
function selectorList(prelude) {
  const selectors = [''];
  let depth = 0;
  for (const char of prelude) {
    if (char === '(') depth += 1;
    if (char === ')') depth -= 1;
    if (char === ',' && depth === 0) selectors.push('');
    else selectors[selectors.length - 1] += char;
  }
  return selectors.map((selector) => selector.trim());
}

// Shared presentation lives in ferramenta-family. The one exception is the
// homepage's own specimens: every rule there names an `fgr-` class, so nothing
// reusable or global can grow in homepage/app.
const specimenStyles = join('components', 'homepage-specimens.css');
const appFiles = await readdir(new URL('../app/', import.meta.url), { recursive: true });
for (const file of appFiles.filter((name) => name.endsWith('.css'))) {
  assert.equal(file, specimenStyles, `Use the shared theme instead of homepage/app/${file}`);
  const css = await readFile(new URL(`../app/${file}`, import.meta.url), 'utf8');
  const unscoped = css
    .replaceAll(/\/\*[\s\S]*?\*\//g, '')
    .split('{')
    .slice(0, -1)
    .map((chunk) =>
      chunk.slice(Math.max(chunk.lastIndexOf('}'), chunk.lastIndexOf(';')) + 1).trim()
    )
    .filter((prelude) => !prelude.startsWith('@'))
    .flatMap((prelude) => selectorList(prelude))
    .filter((selector) => !/\.fgr-/.test(selector));
  assert.deepEqual(unscoped, [], `${file}: scope these selectors to an fgr- class`);
}

const repository = fileURLToPath(new URL('../../', import.meta.url));
const output = new URL('../build/client/', import.meta.url);
const readJson = async (path) => JSON.parse(await readFile(new URL(path, import.meta.url), 'utf8'));
const site = await readJson('../app/data/site.json');
const coverage = await readJson('../app/data/coverage.json');
const samples = await readJson('../app/data/diagram-samples.json');
const grouped = (value) => value.toLocaleString('en-US');

for (const page of expectedPages) {
  const source = await readFile(new URL(page, output), 'utf8');
  const html = source.replaceAll(/<!--[\s\S]*?-->/g, '');
  assert.equal([...html.matchAll(/<h1\b/g)].length, 1, `${page} needs exactly one h1`);
  assert.equal(
    [...html.matchAll(/<main\b|role="main"/g)].length,
    1,
    `${page} needs exactly one main landmark`
  );
  // Every page wears the Monografie header and footer, never Ardo's or the Ferra ones.
  const fragments = ['class="mono-header"', 'class="mono-footer"', 'rel="icon"'];
  if (page !== 'index.html')
    fragments.push('class="ferra-guide-navigation"', 'aria-label="Toggle menu"', 'fgr-docs');
  const absent = fragments.filter((fragment) => !html.includes(fragment));
  const stray = ['class="ardo-header', 'class="ardo-footer', 'ferra-header', 'ferra-footer'].filter(
    (fragment) => html.includes(fragment)
  );
  assert.deepEqual([absent, stray], [[], []], `${page}: missing or unexpected shell parts`);
  if (page === 'index.html') {
    verifyHomepage(html);
    continue;
  }
  const blocks = [...html.matchAll(/<pre\b[^>]*>/g)];
  assert(
    blocks.every(([tag]) => tag.includes('tabindex="0"')),
    `${page} has a code block without keyboard scrolling`
  );
  const tables = [...html.matchAll(/<table\b/g)];
  const regions = [
    ...html.matchAll(/<div\s[^>]*role="region"[^>]*tabindex="0"[^>]*>[\s\S]*?<table\b/g),
  ];
  assert.equal(
    regions.length,
    tables.length,
    `${page} has a table without a keyboard-scrollable region`
  );
}

function verifyHomepage(html) {
  const core = samples.renderedWith['@ferrigura/core'];
  const required = [
    `v${site.version}`,
    'class="mono fgr-site"',
    'data-product="ferrigura"',
    'Mermaid diagrams, drawn without a browser.',
    `Real output of @ferrigura/core ${core}`,
    // The drawings that stand on the page before any choice is made.
    'id="homepage-flowchart"',
    'id="homepage-mode-adaptive"',
    'id="homepage-mode-mermaid"',
    'id="homepage-scheme-light"',
    'id="homepage-scheme-dark"',
    // The figures, recomputed from the coverage table.
    `${coverage.types.length}</em> of ${coverage.mermaidTypes}`,
    `${grouped(coverage.agreed)}</em> of ${grouped(coverage.compared)}`,
    `${grouped(coverage.parsed)}</em>`,
    ...coverage.types.map((row) => `${grouped(row.agreed)} of ${grouped(row.compared)}`),
    // The published anchors.
    ...[
      'overview',
      'example',
      'text',
      'flowchart-mode',
      'schemes',
      'checked',
      'alternatives',
      'boundaries',
      'runtimes',
      'family',
      'start',
    ].map((id) => `id="${id}"`),
  ];
  const missing = required.filter((fragment) => !html.includes(fragment));
  assert.deepEqual(missing, [], 'The homepage is missing these fragments');
  assert.doesNotMatch(html, /<foreignObject\b/, 'Homepage drawings must keep their labels as text');
}

// Dogfood the actual checker for pages, fragments, assets and the project base.
execFileSync(
  'ferrolink',
  ['check', fileURLToPath(output), '--site-url', 'https://ferrigura.dev/'],
  { cwd: repository, stdio: 'inherit' }
);
console.log(
  `Verified the Monografie shell on all ${expectedPages.length} pages and the homepage's drawings from @ferrigura/core ${samples.renderedWith['@ferrigura/core']}.`
);
