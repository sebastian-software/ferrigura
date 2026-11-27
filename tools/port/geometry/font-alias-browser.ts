/** Compare real embedded SVGs with test-only host aliases for each registered design. */
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { chromium } from 'playwright';
import { repositoryRoot } from '../reference-root.ts';

execFileSync(
  'cargo',
  [
    'test',
    '--locked',
    '--test',
    'fonts',
    'font_alias_browser_fixtures::export_font_alias_browser_fixtures',
    '--',
    '--ignored',
    '--exact',
  ],
  { cwd: repositoryRoot, stdio: 'inherit' }
);
const faces = [
  ['400', 'normal', 'HostRegular', 'LexicaUltralegible-Regular.ttf'],
  ['700', 'normal', 'HostBold', 'LexicaUltralegible-Bold.ttf'],
  ['400', 'italic', 'HostItalic', 'LexicaUltralegible-Italic.ttf'],
  ['700', 'italic', 'HostBoldItalic', 'LexicaUltralegible-BoldItalic.ttf'],
  ['500', 'normal', 'HostArrows', 'NotoSansMath-Regular-Arrows.ttf'],
  ['300', 'normal', 'HostEmoji', 'TwemojiMozilla-COLRv0-Emoji.ttf'],
];
const hostCss = (
  await Promise.all(
    faces.map(async ([weight, style, family, file]) => {
      const bytes = await readFile(join(repositoryRoot, 'crates/ferrigura-text/fonts', file));
      return `@font-face{font-family:${family};font-weight:${weight};font-style:${style};src:url(data:font/ttf;base64,${bytes.toString('base64')})}`;
    })
  )
).join('');
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  for (const file of ['sequence', 'plain', 'markdown']) {
    const svg = await readFile(
      join(repositoryRoot, `target/font-alias-browser/${file}.svg`),
      'utf8'
    );
    await page.setContent(svg);
    await page.evaluate('globalThis.__name = (target) => target;');
    const capture = () =>
      page.evaluate(async () => {
        await document.fonts.ready;
        return [...document.querySelectorAll<SVGTextContentElement>('text,tspan')]
          .filter((node) => node.textContent?.trim())
          .map((node) => {
            const box = node.getBBox();
            return {
              text: node.textContent,
              nested: Boolean(node.querySelector('tspan')),
              box: [box.x, box.y, box.width, box.height],
              advance: node.getComputedTextLength(),
            };
          });
      });
    const platformFonts = async () => {
      const count = await page.evaluate(() => {
        const nodes = [...document.querySelectorAll<SVGTextContentElement>('text,tspan')].filter(
          (node) =>
            [...node.childNodes].some(
              (child) => child.nodeType === Node.TEXT_NODE && child.textContent?.trim()
            )
        );
        nodes.forEach((node, index) => node.setAttribute('data-alias-proof', String(index)));
        return nodes.length;
      });
      const cdp = await page.context().newCDPSession(page);
      await cdp.send('DOM.enable');
      await cdp.send('CSS.enable');
      const { root } = await cdp.send('DOM.getDocument');
      const result = [];
      for (let index = 0; index < count; index++) {
        const { nodeId } = await cdp.send('DOM.querySelector', {
          nodeId: root.nodeId,
          selector: `[data-alias-proof="${index}"]`,
        });
        const { fonts } = await cdp.send('CSS.getPlatformFontsForNode', { nodeId });
        result.push(
          fonts.map((font) => ({ name: font.postScriptName, custom: font.isCustomFont }))
        );
      }
      await cdp.detach();
      return result;
    };
    const embedded = await capture();
    if (file === 'markdown') {
      assert.ok(
        embedded.some((node) => node.nested),
        'Markdown must cover an enclosing text container'
      );
    }
    const embeddedFonts = await platformFonts();
    assert.ok(embedded.length, `${file} must render text`);
    const count = await page.evaluate(
      ({ faces, hostCss }) => {
        const svg = document.querySelector('svg')!;
        const styles = [...svg.querySelectorAll('style')];
        const mappings: [string, string][] = [];
        for (const style of styles) {
          style.textContent = style.textContent!.replace(/@font-face\{[^}]*\}/g, (rule) => {
            const family = /font-family:"([^"]+)"/.exec(rule)?.[1];
            const weight = /font-weight:([^;]+);/.exec(rule)?.[1];
            const faceStyle = /font-style:([^;]+);/.exec(rule)?.[1];
            const face = faces.find((item) => item[0] === weight && item[1] === faceStyle);
            if (!family || !face || !rule.includes('src:url(data:font/ttf;base64,'))
              throw Error('Unexpected embedded face');
            mappings.push([family, face[2]]);
            return '';
          });
        }
        if (!mappings.length || new Set(mappings.map((item) => item[0])).size !== mappings.length)
          throw Error('Missing or duplicate embedded face');
        const replace = (value: string) =>
          mappings.reduce((result, [from, to]) => result.replaceAll(from, to), value);
        for (const style of styles) style.textContent = replace(style.textContent!);
        for (const node of [svg, ...svg.querySelectorAll('*')])
          for (const attr of node.attributes) {
            if (mappings.some(([from]) => attr.value.includes(from))) {
              if (!['style', 'font-family'].includes(attr.name))
                throw Error(`Unexpected family reference in ${attr.name}`);
              node.setAttribute(attr.name, replace(attr.value));
            }
          }
        const host = document.createElement('style');
        host.textContent = hostCss;
        document.head.append(host);
        return mappings.length;
      },
      { faces, hostCss }
    );
    assert.ok(count, `${file} must have embedded designs`);
    const named = await capture();
    const namedFonts = await platformFonts();
    assert.deepEqual(namedFonts, embeddedFonts, `${file} must use the same physical fonts`);
    assert.ok(
      namedFonts.every((fonts) => fonts.length > 0 && fonts.every((font) => font.custom)),
      `${file} must use registered fonts`
    );
    assert.equal(named.length, embedded.length);
    let maxBoxDelta = 0;
    let maxAdvanceDelta = 0;
    for (let i = 0; i < embedded.length; i++) {
      assert.equal(named[i].text, embedded[i].text, `${file} text ${i}`);
      assert.equal(named[i].nested, embedded[i].nested, `${file} nesting ${i}`);
      for (let j = 0; j < 4; j++) {
        const delta = Math.abs(named[i].box[j] - embedded[i].box[j]);
        maxBoxDelta = Math.max(maxBoxDelta, delta);
        assert.equal(delta, 0, `${file} text ${i} box ${j}`);
      }
      const delta = Math.abs(named[i].advance - embedded[i].advance);
      maxAdvanceDelta = Math.max(maxAdvanceDelta, delta);
      assert.equal(delta, 0, `${file} text ${i} advance`);
    }
    console.log(
      `PASS ${file}: ${embedded.length} text elements (${embedded.filter((node) => node.nested).length} containers), ${count} designs; max box delta ${maxBoxDelta}, advance delta ${maxAdvanceDelta}`
    );
  }
} finally {
  await browser.close();
}
