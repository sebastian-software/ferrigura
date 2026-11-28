/* eslint-disable security/detect-non-literal-fs-filename -- Paths are fixed files in this repository. */
// Renders the diagrams of the repository's Markdown (README, docs/) from their
// sources in docs/images/*.mmd, so GitHub and the site show Ferrigura's own
// drawing instead of rendering the text with Mermaid. Each file carries both
// color schemes and follows the reader's. With --check, the committed files
// are compared with a fresh render (published-output.mjs).
import assert from 'node:assert/strict';
import { readdir, readFile, writeFile } from 'node:fs/promises';

import { compareOutput } from './published-output.mjs';
import { createSourceRenderer } from './source-renderer.mjs';

const check = process.argv.includes('--check');
const directory = new URL('../../docs/images/', import.meta.url);
const manifest = new URL('diagrams.json', directory);
const platform = `${process.platform}-${process.arch}`;
const recorded = check ? JSON.parse(await readFile(manifest, 'utf8')) : undefined;
const exact = !check || recorded.platform === platform;

const entries = await readdir(directory);
const sources = entries.filter((entry) => entry.endsWith('.mmd')).toSorted();
assert(sources.length > 0, 'docs/images has no diagram sources');
const svgs = entries.filter((entry) => entry.endsWith('.svg')).toSorted();
if (check) {
  const expected = sources.map((entry) => entry.replace(/\.mmd$/, '.svg'));
  assert.deepEqual(svgs, expected, 'docs/images: every .mmd needs its .svg and no other SVG');
}

const renderer = await createSourceRenderer();
try {
  for (const entry of sources) {
    const name = entry.replace(/\.mmd$/, '');
    const source = await readFile(new URL(entry, directory), 'utf8');
    const svg = renderer.render(source, { colorScheme: 'auto', id: `docs-${name}` });
    assert.doesNotMatch(svg, /<foreignObject\b|<script\b/, `${name} contains HTML or script`);
    const target = new URL(`${name}.svg`, directory);
    if (check) {
      const actual = await readFile(target, 'utf8');
      const message = `docs/images/${name}.svg drifted from its source; run pnpm docs-diagrams:write`;
      compareOutput({ actual, expected: svg, label: `${name}.svg`, exact, message });
    } else {
      await writeFile(target, svg);
    }
  }
  if (!check) await writeFile(manifest, `${JSON.stringify({ platform }, null, 2)}\n`);
  console.log(
    `${check ? 'Verified' : 'Wrote'} ${sources.length} documentation diagrams from the repository source${exact ? '' : ` (${platform}, numbers within 0.01 of ${recorded.platform})`}.`
  );
} finally {
  await renderer.remove();
}
