/** Checks owned local-font metrics, exact designs and default variable axes in Chromium. */
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
const root = fileURLToPath(new URL('../../../', import.meta.url));
const fontDir = join(root, 'crates/ferrigura-text/fonts');
const fixtureDir = join(root, 'crates/ferrigura-text/tests/fixtures');
const target = process.env.CARGO_TARGET_DIR
  ? resolve(root, process.env.CARGO_TARGET_DIR)
  : join(root, 'target');
const output = join(root, 'target/geometry/registered-fonts');
const sample = 'Hamburgefontsiv AVATAR iii WWW';
type Style = 'normal' | 'italic';
type Font = { family: string; path: string; weight?: number; style?: Style };
type Run = { family: string; text: string; weight?: number; style?: Style };
type Case = { name: string; size: number; fonts: Font[]; runs: Run[]; fallbacks?: string[] };
const primary: Font = { family: 'Primary', path: join(fontDir, 'LexicaUltralegible-Regular.ttf') };
const arrows: Font = { family: 'Fallback', path: join(fontDir, 'NotoSansMath-Regular-Arrows.ttf') };
const cases: Case[] = [];
for (const size of [16, 32]) {
  for (const [name, path] of [
    ['mono', join(fontDir, 'AtkinsonHyperlegibleMono-Regular.ttf')],
    ['cff', join(fixtureDir, 'SourceCodePro-Regular.otf')],
  ]) {
    cases.push({
      name: `${name}-${size}`,
      size,
      fonts: [{ family: 'Primary', path }],
      runs: [{ family: 'Primary', text: sample }],
    });
  }
  for (const weight of [400, 700])
    cases.push({
      name: `variable-${weight}-${size}`,
      size,
      fonts: [{ family: 'Primary', path: join(fixtureDir, 'Inter-DefaultAxes.ttf'), weight }],
      runs: [{ family: 'Primary', text: sample, weight }],
    });
  cases.push({
    name: `mixed-designs-${size}`,
    size,
    fonts: [
      { ...primary, weight: 600 },
      { ...arrows, weight: 500 },
      { family: 'Fallback', path: join(fixtureDir, 'SourceCodePro-Regular.otf'), weight: 800 },
    ],
    fallbacks: ['Fallback'],
    runs: [
      { family: 'Primary', text: '→', weight: 700 },
      { family: 'Primary', text: ' →', style: 'italic' },
      { family: 'Primary', text: ' →' },
    ],
  });
}
for (const [name, size, path] of [
  ['cff-tie', 62.5, join(fixtureDir, 'SourceCodePro-Regular.otf')],
  ['mono-tie', 62.5, join(fontDir, 'AtkinsonHyperlegibleMono-Regular.ttf')],
  ['sans-size-stage', 165.79, join(fontDir, 'LexicaUltralegible-Regular.ttf')],
  ['sans-tie', 30, join(fontDir, 'LexicaUltralegible-Regular.ttf')],
] as const)
  cases.push({
    name,
    size,
    fonts: [{ family: 'Primary', path }],
    runs: [{ family: 'Primary', text: sample }],
  });
cases.push({
  name: 'all-fallback',
  size: 16,
  fonts: [primary, arrows],
  fallbacks: ['Fallback'],
  runs: [{ family: 'Primary', text: '→' }],
});
cases.push({
  name: 'mono-italic',
  size: 18,
  fonts: [
    {
      family: 'Primary',
      path: join(fontDir, 'AtkinsonHyperlegibleMono-Italic.ttf'),
      style: 'italic',
    },
  ],
  runs: [{ family: 'Primary', text: 'jf AVATAR', style: 'italic' }],
});
execFileSync('cargo', ['build', '-q', '-p', 'ferrigura-text', '--example', 'registered-font'], {
  cwd: root,
  stdio: 'inherit',
});
await mkdir(output, { recursive: true });
const browser = await chromium.launch({ args: ['--font-render-hinting=none'] });
const results: unknown[] = [];
let failures = 0;
try {
  const page = await browser.newPage();
  for (const entry of cases) {
    const probe = spawnSync(join(target, 'debug/examples/registered-font'), [], {
      cwd: root,
      input: JSON.stringify(entry),
      encoding: 'utf8',
    });
    if (probe.status !== 0) throw new Error(`${entry.name}: ${probe.stderr}`);
    const expected = JSON.parse(probe.stdout) as {
      svg: string;
      advance: number;
      x: number;
      width: number;
      ascent: number;
      descent: number;
    };
    await writeFile(join(output, `${entry.name}.svg`), expected.svg);
    await page.setContent(expected.svg);
    const actual = await page.evaluate(async () => {
      const text = document.querySelector('text');
      if (!text) throw new Error('probe has no text');
      text.getBBox();
      await document.fonts.ready;
      const box = text.getBBox();
      return {
        advance: text.getComputedTextLength(),
        x: box.x,
        width: box.width,
        ascent: 100 - box.y,
        descent: box.y + box.height - 100,
      };
    });
    const differences = (['advance', 'x', 'width', 'ascent', 'descent'] as const)
      .filter((key) => Math.abs(actual[key] - expected[key]) > 0.02)
      .map((key) => `${key}: Rust ${expected[key]}, Chromium ${actual[key]}`);
    if (differences.length) failures++;
    results.push({
      name: entry.name,
      expected: {
        advance: expected.advance,
        x: expected.x,
        width: expected.width,
        ascent: expected.ascent,
        descent: expected.descent,
      },
      actual,
      differences,
    });
    console.log(
      `${differences.length ? 'FAIL' : 'PASS'} ${entry.name}${differences.length ? `: ${differences.join('; ')}` : ''}`
    );
  }
} finally {
  await browser.close();
}
await writeFile(join(output, 'results.json'), `${JSON.stringify(results, null, 2)}\n`);
if (failures) throw new Error(`${failures} registered-font cases disagree with Chromium`);
console.log(`${cases.length} registered-font cases match Chromium`);
