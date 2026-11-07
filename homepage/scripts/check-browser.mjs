/* eslint-disable security/detect-non-literal-fs-filename -- Vite and axe paths are package-owned; pages come from the site route list. */
import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { preview } from 'vite';

import { expectedPages, pagePath } from './site-pages.mjs';

const homepageRoot = fileURLToPath(new URL('..', import.meta.url));
const require = createRequire(import.meta.url);
const axeSource = await readFile(require.resolve('axe-core/axe.min.js'), 'utf8');
const requestedPort = Number(process.env.FERRIGURA_PREVIEW_PORT ?? 0);
assert(
  Number.isInteger(requestedPort) && requestedPort >= 0 && requestedPort <= 65_535,
  'FERRIGURA_PREVIEW_PORT must be a valid TCP port'
);
const selectors = {
  nav: '.mono-header nav',
  // The header's own menu; documentation pages hand narrow screens to their drawer instead.
  headerMenu: '.mono-header .mono-menu-button',
  menu: 'button[aria-label="Toggle menu"]',
  search: 'button[aria-label="Search"]',
};

let browser;
let previewServer;
try {
  previewServer = await preview({
    configFile: false,
    root: homepageRoot,
    logLevel: 'silent',
    base: '/',
    build: { outDir: 'build/client' },
    preview: {
      host: '127.0.0.1',
      port: requestedPort,
      strictPort: requestedPort !== 0,
    },
  });
  const address = previewServer.httpServer.address();
  assert(
    address && typeof address !== 'string',
    'Vite preview did not expose a listening TCP address'
  );
  const origin = `http://127.0.0.1:${address.port}`;
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ colorScheme: 'light' });
  const runtimeErrors = [];
  page.on('pageerror', (error) => runtimeErrors.push(error.message));
  for (const pageFile of expectedPages) {
    for (const viewport of [
      { name: 'phone-small', width: 320, height: 760 },
      { name: 'phone-compact', width: 375, height: 812 },
      { name: 'phone', width: 390, height: 844 },
      { name: 'phone-wide', width: 640, height: 900 },
      { name: 'tablet', width: 768, height: 1024 },
      { name: 'landscape', width: 844, height: 390 },
      { name: 'tablet-wide', width: 1024, height: 900 },
      { name: 'desktop', width: 1440, height: 900 },
    ]) {
      await verifyViewport({ page, pageFile, viewport, origin });
    }
  }

  await verifyExample(page, origin);
  await verifyNavigation(page, origin);
  await verifyHeaderMenu(page, origin);
  await verifySearch(page, origin);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(new URL('/', origin).href, { waitUntil: 'load' });
  const motion = await page
    .locator('.mono, .mono *')
    .evaluateAll((elements) =>
      elements
        .map((element) => getComputedStyle(element).animationDuration)
        .filter((value) => value !== '0s')
    );
  assert.deepEqual(motion, [], 'Reduced motion still animates the page');
  assert.deepEqual(runtimeErrors, [], 'The site raised browser runtime errors');
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await capture(page, origin);
  console.log(
    'Verified all pages at eight widths, the diagram choices and their files, both menus, search, focus return, reduced motion and runtime errors.'
  );
} finally {
  await browser?.close();
  await previewServer?.close();
}

async function verifyExample(page, origin) {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(new URL('/', origin).href, { waitUntil: 'load' });
  const example = page.locator('#example');
  const choice = example.getByRole('group', { name: 'Diagram type', exact: true });
  for (const [id, label] of [
    ['sequence', 'Sequence'],
    ['er', 'Entity relationship'],
    ['state', 'State'],
    ['flowchart', 'Flowchart'],
  ]) {
    await choice.getByRole('button', { name: label, exact: true }).click();
    await example.locator(`svg[id="homepage-${id}"]`).waitFor();
    assert.equal(
      await choice.getByRole('button', { name: label, exact: true }).getAttribute('aria-pressed'),
      'true',
      `The ${label} choice is not pressed`
    );
    assert((await example.locator('svg text').count()) > 0, 'Diagram labels must remain SVG text');
    assert.equal(await example.locator('foreignObject').count(), 0);
    const file = await example
      .getByRole('link', { name: 'Open the SVG file' })
      .getAttribute('href');
    const response = await page.request.get(new URL(file, origin).href);
    const body = await response.text();
    assert(
      response.ok() && body.includes(`id="homepage-${id}"`),
      `The ${label} file link does not lead to its SVG`
    );
  }
}
/** Below 961px the homepage folds its links into the header menu, which Escape closes. */
async function verifyHeaderMenu(page, origin) {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(new URL('/', origin).href, { waitUntil: 'load' });
  const button = page.locator(selectors.headerMenu);
  await button.click();
  const link = page.locator('.mono-menu').getByRole('link', { name: 'Diagram types', exact: true });
  await link.waitFor();
  const bounds = await link.boundingBox();
  assert(
    bounds && bounds.x >= 0 && bounds.x + bounds.width <= 390,
    'Menu link leaves the viewport'
  );
  await page.keyboard.press('Escape');
  await link.waitFor({ state: 'hidden' });
  assert(
    await button.evaluate((element) => element === document.activeElement),
    'Header menu focus was not restored'
  );
}
async function verifyNavigation(page, origin) {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(new URL('/guide/getting-started/', origin).href, { waitUntil: 'load' });
  await page.getByRole('heading', { name: 'Getting started', level: 1, exact: true }).waitFor();
  const menu = page.locator(selectors.menu);
  await focusMenuByKeyboard(page, menu);
  await page.keyboard.press('Enter');
  await page.getByRole('dialog').waitFor();
  await page.keyboard.press('Escape');
  await page.getByRole('dialog').waitFor({ state: 'detached' });
  assert(
    await menu.evaluate((element) => element === document.activeElement),
    'Menu focus was not restored'
  );
  await page.keyboard.press('Enter');
  await page
    .getByRole('dialog')
    .getByRole('link', { name: 'Getting started', exact: true })
    .click();
  await page.waitForURL((url) => url.pathname.startsWith('/guide/getting-started'));
  await page.getByRole('dialog').waitFor({ state: 'detached' });
  await verifyLandscape(page, origin);
}
async function focusMenuByKeyboard(page, menu) {
  let reached = false;
  for (let i = 0; i < 40; i++) {
    await page.keyboard.press('Tab');
    if (await menu.evaluate((element) => element === document.activeElement)) {
      reached = true;
      break;
    }
  }
  assert(reached, 'Keyboard navigation does not reach the menu');
}
async function verifyLandscape(page, origin) {
  await page.setViewportSize({ width: 844, height: 390 });
  await page.goto(new URL('/reference/rust/', origin).href, { waitUntil: 'load' });
  const table = page
    .locator('div[role="region"][tabindex="0"]')
    .filter({ has: page.locator('table') })
    .first();
  await table.focus();
  assert(
    await table.evaluate((element) => element === document.activeElement),
    'Markdown tables are not keyboard focusable'
  );
  await page.locator(selectors.menu).focus();
  await page.keyboard.press('Enter');
  const lastLink = page.getByRole('dialog').getByRole('link').last();
  await lastLink.focus();
  const bounds = await lastLink.boundingBox();
  assert(
    bounds && bounds.y >= 0 && bounds.y + bounds.height <= 390,
    'Landscape menu link leaves the viewport'
  );
  await page.keyboard.press('Escape');
}
async function verifySearch(page, origin) {
  for (const width of [320, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    // Search lives in the documentation header; the homepage keeps its own links.
    await page.goto(new URL('/guide/getting-started/', origin).href, { waitUntil: 'load' });
    const search = page.locator(selectors.search);
    await search
      .locator('kbd')
      .first()
      .evaluate((element) => {
        element.textContent = 'Ctrl';
      });
    await search.hover();
    const hint = await search.locator('span[aria-hidden="true"]').boundingBox();
    assert(hint && hint.x >= 0 && hint.x + hint.width <= width, 'Search hint leaves the viewport');
    await search.click();
    const dialog = page.getByRole('dialog');
    await dialog.locator('input').fill('bundled fonts');
    await dialog.locator('a[href*="/reference/rust"]').first().waitFor();
    await page.keyboard.press('Escape');
    await dialog.waitFor({ state: 'detached' });
    assert(
      await search.evaluate((element) => element === document.activeElement),
      'Search focus was not restored'
    );
  }
}
async function capture(page, origin) {
  await mkdir(join(tmpdir(), 'ferrigura-review'), { recursive: true });
  for (const [path, label] of [
    ['/', 'home'],
    ['/guide/getting-started/', 'guide'],
    ['/reference/rust/', 'reference'],
    ['/diagrams/overview/', 'evidence'],
  ]) {
    for (const width of [390, 1440]) {
      await page.setViewportSize({ width, height: 1000 });
      await page.goto(new URL(path, origin).href, { waitUntil: 'load' });
      await page.screenshot({
        path: join(tmpdir(), `ferrigura-review/${label}-${width}.png`),
        fullPage: true,
      });
    }
  }
}

async function verifyViewport({ page, pageFile, viewport, origin }) {
  await page.setViewportSize({ width: viewport.width, height: viewport.height });
  await page.goto(new URL(pagePath(pageFile), origin).href, { waitUntil: 'load' });
  const layout = await page.evaluate(() => ({
    width: globalThis.document.documentElement.clientWidth,
    pageWidth: Math.max(
      globalThis.document.documentElement.scrollWidth,
      globalThis.document.body.scrollWidth
    ),
  }));
  assert(
    layout.pageWidth <= layout.width + 1,
    `${pageFile} has horizontal page overflow at ${viewport.name} width ${viewport.width}px: ${layout.pageWidth}px`
  );

  await verifyHeader(page, pageFile, viewport);
  if (
    [
      'index.html',
      'guide/getting-started/index.html',
      'reference/rust/index.html',
      'diagrams/overview/index.html',
    ].includes(pageFile)
  )
    await verifyAccessibility(page, pageFile, viewport);
  console.log(`Verified ${pageFile} at ${viewport.name} (${viewport.width}px).`);
}
/** Links on desktop; below 961px the homepage's header menu or the documentation drawer. */
async function verifyHeader(page, pageFile, viewport) {
  const home = pageFile === 'index.html';
  const navVisible = await page.locator(selectors.nav).isVisible();
  const headerMenuVisible = await page.locator(selectors.headerMenu).isVisible();
  const drawerVisible = home ? false : await page.locator(selectors.menu).isVisible();
  if (!home) {
    assert(
      await page.locator(selectors.search).isVisible(),
      'Documentation pages should keep search visible at each checked viewport'
    );
  }
  if (viewport.width >= 961) {
    assert(navVisible, `${pageFile} hides the header links at ${viewport.name} width`);
    assert(!headerMenuVisible, `${pageFile} shows the header menu at ${viewport.name} width`);
    // The documentation drawer stands in for the sidebar until the sidebar fits.
    if (viewport.name === 'desktop')
      assert(!drawerVisible, `${pageFile} shows the documentation drawer on desktop`);
  } else {
    assert(!navVisible, `${pageFile} shows the header links at ${viewport.name} width`);
    assert(
      home ? headerMenuVisible : drawerVisible && !headerMenuVisible,
      `${pageFile} shows the wrong menu at ${viewport.name} width`
    );
  }
}
async function verifyAccessibility(page, pageFile, viewport) {
  await page.addScriptTag({ content: axeSource });
  const results = await page.evaluate(async () =>
    globalThis.axe.run(globalThis.document, {
      runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'] },
    })
  );
  if (results.violations.length === 0) return;
  const details = results.violations
    .map((violation) => {
      const targets = violation.nodes
        .map((node) => `  ${node.target.join(', ')}: ${node.failureSummary}`)
        .join('\n');
      return `${violation.id} (${violation.impact}): ${violation.help}\n${targets}`;
    })
    .join('\n');
  throw new Error(`Accessibility violations on ${pageFile} at ${viewport.name} width:\n${details}`);
}
