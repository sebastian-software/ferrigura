/** Compare accessibility metadata with the pinned reference independently of geometry.
 * Run with `pnpm exec tsx geometry/accessibility.ts` after setting up the reference and Chromium.
 */
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { buildRenderExample, renderFerrigura, renderReference, root, withPage } from './common.js';

type Case = { path: string; mode: string };
type Metadata = {
  role: string | null;
  type: string | null;
  title: string | null;
  description: string | null;
  titleRef: string | null;
  descriptionRef: string | null;
  directTitles: number;
  directDescriptions: number;
};
const cases = JSON.parse(
  await readFile(`${root}/crates/ferrigura/tests/accessibility-cases.json`, 'utf8')
) as Case[];
const decodedDescription = new Set([
  'e2e/diagrams/flowchart/regressions/routing-review-cycle-bt.mmd',
  'tools/port/geometry/probes/swimlanes/p01-direction-br.mmd',
  'e2e/diagrams/pie/pr-check/should-render-a-pie-diagram-with-showdata.mmd',
]);
const unmatchedDifferences = new Set(decodedDescription);
buildRenderExample();
await withPage('const measure = () => ({});', async (page) => {
  for (const { path, mode } of cases) {
    let source = await readFile(`${root}/${path}`, 'utf8');
    if (source.startsWith('---\n')) source = source.split('\n---\n').slice(1).join('\n---\n');
    source = source
      .split('\n')
      .filter((line) => !line.trimStart().startsWith('%%'))
      .join('\n')
      .trimStart();
    if (!source.includes('\n')) source += '\n';
    const split = source.indexOf('\n');
    const input =
      mode === 'unsupported'
        ? source
        : mode === 'eventmodeling'
          ? `${source.slice(0, split)} accTitle: Alpha & title\n${source.slice(split + 1)}\ntf 99 ui X accDescr: A < B & C\n`
          : `${source.slice(0, split)}\naccTitle: Alpha & title\naccDescr: A < B & C\n${source.slice(split + 1)}`;
    const variants: [string, boolean][] = [
      [input, mode !== 'unsupported'],
      [source, false],
    ];
    if (path === 'tools/port/geometry/probes/railroad/c18-peg.mmd') {
      variants.push(['railroad-beta\naccTitle: Named\naccTitle:\nr = terminal("a") ;\n', false]);
    }
    for (const [inputText, hasMetadata] of variants) {
      // Mermaid shares diagram DBs across renders. A fresh page keeps a previous case from supplying
      // metadata to the next one (the C4 common description in particular is not reset upstream).
      await page.reload();
      await page.waitForFunction(() => (window as unknown as { ready?: boolean }).ready === true);
      const reference = await renderReference(page, 'a11y-check', inputText, {
        now: 1_760_000_000_000,
        seed: 1,
      });
      assert.ok(reference.svg, `${path}: reference failed: ${reference.error}`);
      const rust = renderFerrigura(inputText, 'a11y-check', 1_760_000_000_000, 1);
      assert.ok('svg' in rust, `${path}: ferrigura failed: ${'error' in rust ? rust.error : ''}`);
      const extract = async (svg: string): Promise<Metadata> =>
        page.evaluate((value) => {
          // HTML parsing also handles the reference's foreignObject in eventmodeling.
          const document = new DOMParser().parseFromString(value, 'text/html');
          const root = document.querySelector('svg');
          if (!root) throw new Error('missing SVG');
          const titleId = root.getAttribute('aria-labelledby');
          const descId = root.getAttribute('aria-describedby');
          const directTitles = root.querySelectorAll(':scope > title');
          const directDescriptions = root.querySelectorAll(':scope > desc');
          const titles = titleId
            ? [...root.querySelectorAll('[id]')].filter((node) => node.id === titleId)
            : [];
          const descriptions = descId
            ? [...root.querySelectorAll('[id]')].filter((node) => node.id === descId)
            : [];
          if (titleId && (titles.length !== 1 || titles[0].localName !== 'title'))
            throw new Error('aria-labelledby does not resolve locally');
          if (descId && (descriptions.length !== 1 || descriptions[0].localName !== 'desc'))
            throw new Error('aria-describedby does not resolve locally');
          return {
            directTitles: directTitles.length,
            directDescriptions: directDescriptions.length,
            role: root.getAttribute('role'),
            type: root.getAttribute('aria-roledescription'),
            title: titles[0]?.textContent ?? null,
            description: descriptions[0]?.textContent ?? null,
            titleRef: titleId,
            descriptionRef: descId,
          };
        }, svg);
      const expected = await extract(reference.svg);
      const actual = await extract(rust.svg);
      // These three renderers decode the reference's literal entity text in the accessible
      // description. Check both exact values so a new mismatch cannot be absorbed by the exception.
      if (hasMetadata && decodedDescription.has(path)) {
        assert.equal(expected.description, 'A &lt; B &amp; C', `${path}: reference description`);
        assert.equal(actual.description, 'A < B & C', `${path}: ferrigura description`);
        expected.description = actual.description;
        unmatchedDifferences.delete(path);
      }
      assert.deepEqual(actual, expected, `${path}: ${hasMetadata ? 'metadata' : 'no metadata'}`);
    }
    console.log(`✓ ${path}`);
  }
  assert.equal(
    unmatchedDifferences.size,
    0,
    `missing expected cases: ${[...unmatchedDifferences].join(', ')}`
  );
});
