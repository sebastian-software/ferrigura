/**
 * Checks the actual browser font of all 22 custom diagram renderers, with two different primary
 * files registered under the same alias. Bold and italic use distinct designs, so an inherited
 * regular family cannot pass merely because an @font-face declaration exists.
 *
 * Usage: `pnpm -C tools/port test:custom-font-renderers`
 * The public Rust fixtures write the SVGs; Chromium checks loaded families and per-role designs.
 * Results are written to target/custom-font-renderers/computed-fonts.json.
 */
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import { chromium } from 'playwright';

import { repositoryRoot } from '../reference-root.ts';

execFileSync(
  'cargo',
  [
    'test',
    '--locked',
    '--test',
    'custom_font_renderers',
    'export_custom_renderer_browser_fixtures',
    '--',
    '--ignored',
    '--exact',
  ],
  { cwd: repositoryRoot, stdio: 'inherit' }
);

const directory = join(repositoryRoot, 'target/custom-font-renderers');
const files: string[] = JSON.parse(await readFile(join(directory, 'fixtures.json'), 'utf8'));
assert.equal(files.length, 44, '22 custom types must each render with both primary files');
assert.equal(new Set(files).size, 44, 'fixture names must be unique');
const results = [];
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  for (const file of files) {
    await page.setContent(await readFile(join(directory, file), 'utf8'));
    const proof = await page.evaluate(async () => {
      await document.fonts.ready;
      const loaded = [...document.fonts].map((face) => ({
        family: face.family.replaceAll('"', ''),
        status: face.status,
        weight: face.weight,
        style: face.style,
      }));
      const text = [...document.querySelectorAll<SVGTextContentElement>('text,tspan')]
        .filter((node) =>
          [...node.childNodes].some(
            (child) => child.nodeType === Node.TEXT_NODE && child.textContent?.trim()
          )
        )
        .map((node) => {
          const css = getComputedStyle(node);
          const box = node.getBBox();
          const family = css.fontFamily.replaceAll('"', '').split(',')[0].trim();
          // The fixture registry has normal/bold designs at 400/700 and both italic designs.
          const expectedWeight = Number(css.fontWeight) > 500 ? 700 : 400;
          return {
            text: node.textContent,
            family,
            weight: css.fontWeight,
            style: css.fontStyle,
            width: box.width,
            height: box.height,
            loaded: loaded.some((face) => face.family === family && face.status === 'loaded'),
            design: loaded.some(
              (face) =>
                face.family === family &&
                face.style === css.fontStyle &&
                Number(face.weight) === expectedWeight
            ),
          };
        });
      return { loaded, text };
    });
    const failures = proof.text.filter((text) => !text.loaded || !text.design);
    results.push({ file, failures, ...proof });
    console.log(
      `${failures.length ? 'FAIL' : 'PASS'} ${file}: ${proof.text.length} text runs; ${proof.loaded.length} embedded designs`
    );
  }
} finally {
  await browser.close();
}
await writeFile(join(directory, 'computed-fonts.json'), JSON.stringify(results, null, 2) + '\n');
for (const result of results) {
  assert.ok(result.text.length > 0, `${result.file} must draw text`);
  assert.deepEqual(
    result.failures,
    [],
    `${result.file} must draw every run in its resolved design`
  );
}
