/** Compare Mermaid's measured placeholders with registered and browser fallback on macOS. */
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const fontDir = join(root, 'crates/ferrigura-text/fonts');
const target = process.env.CARGO_TARGET_DIR
  ? resolve(root, process.env.CARGO_TARGET_DIR)
  : join(root, 'target');
const files = {
  lexica: 'LexicaUltralegible-Regular.ttf',
  mono: 'AtkinsonHyperlegibleMono-Regular.ttf',
} as const;
const texts = { encoded: 'using ﬂ°lt¶ßbrﬂ°gt¶ß', decoded: 'using <br>' } as const;
execFileSync(
  'cargo',
  ['build', '--locked', '-q', '-p', 'ferrigura-text', '--example', 'registered-font'],
  { cwd: root, stdio: 'inherit' }
);
const rust = (file: string, text: string, fallback = false): number => {
  const fonts = [{ family: 'Primary', path: join(fontDir, file) }];
  if (fallback) fonts.push({ family: 'Fallback', path: join(fontDir, files.lexica) });
  const input = {
    size: 16,
    fonts,
    fallbacks: fallback ? ['Fallback'] : [],
    runs: [{ family: 'Primary', text }],
  };
  const result = spawnSync(join(target, 'debug/examples/registered-font'), [], {
    input: JSON.stringify(input),
    encoding: 'utf8',
  });
  if (result.status !== 0) throw new Error(result.stderr);
  return (JSON.parse(result.stdout) as { advance: number }).advance;
};
const browser = await chromium.launch({ args: ['--font-render-hinting=none'] });
try {
  const page = await browser.newPage();
  const css = Object.entries(files)
    .map(
      ([name, file]) =>
        `@font-face{font-family:${name};src:url(data:font/ttf;base64,${readFileSync(join(fontDir, file)).toString('base64')})}`
    )
    .join('');
  await page.setContent(
    `<style>${css}</style><svg xmlns="http://www.w3.org/2000/svg"><text id="probe" font-size="16"></text></svg>`
  );
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('DOM.enable');
  await cdp.send('CSS.enable');
  const { root: documentNode } = await cdp.send('DOM.getDocument');
  const { nodeId } = await cdp.send('DOM.querySelector', {
    nodeId: documentNode.nodeId,
    selector: '#probe',
  });
  for (const [name, file] of Object.entries(files)) {
    for (const [kind, text] of Object.entries(texts)) {
      const browserWidth = await page.evaluate(
        async ({ name, text }) => {
          await document.fonts.load(`16px ${name}`, text);
          const element = document.querySelector('text')!;
          element.setAttribute('font-family', name);
          element.textContent = text;
          return element.getComputedTextLength();
        },
        { name, text }
      );
      const { fonts } = await cdp.send('CSS.getPlatformFontsForNode', { nodeId });
      const missingFallback = fonts.some((font) => !font.isCustomFont);
      const rustWidth = rust(file, text);
      assert.equal(
        missingFallback,
        name === 'mono' && kind === 'encoded',
        `${name} ${kind}: ${JSON.stringify(fonts)}`
      );
      if (missingFallback) assert.ok(Math.abs(browserWidth - rustWidth) > 0.02);
      else
        assert.ok(
          Math.abs(browserWidth - rustWidth) <= 0.02,
          `${name} ${kind}: Rust ${rustWidth}, Chromium ${browserWidth}`
        );
      console.log(
        `${name} ${kind}: Rust ${rustWidth}, Chromium ${browserWidth}, fonts ${JSON.stringify(fonts)}`
      );
    }
  }
  const registeredWidth = rust(files.mono, texts.encoded, true);
  const browserWidth = await page.evaluate(async (text) => {
    const element = document.querySelector('text')!;
    element.setAttribute('font-family', 'mono, lexica');
    element.textContent = text;
    await document.fonts.ready;
    return element.getComputedTextLength();
  }, texts.encoded);
  const { fonts: registeredFonts } = await cdp.send('CSS.getPlatformFontsForNode', { nodeId });
  assert.equal(registeredFonts.length, 2);
  assert.ok(registeredFonts.every((font) => font.isCustomFont));
  assert.ok(Math.abs(browserWidth - registeredWidth) <= 0.02);
  console.log(
    `mono with registered Lexica fallback: Rust ${registeredWidth}, Chromium ${browserWidth}, fonts ${JSON.stringify(registeredFonts)}`
  );
  console.log(`Chromium ${browser.version()}`);
} finally {
  await browser.close();
}
