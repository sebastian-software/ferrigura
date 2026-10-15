/* eslint-disable @typescript-eslint/no-explicit-any */
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';
import { expect, type Page, type TestInfo } from '@playwright/test';
import { Buffer } from 'buffer';
import type { MermaidConfig } from '../../packages/mermaid/src/config.type.js';
import { collectCoverage, startCoverage } from './coverage.js';

interface E2EConfig {
  listUrl?: boolean;
  listId?: string;
  name?: string;
  screenshot?: boolean;
  /** Fail when mermaid renders its syntax-error diagram instead of the requested type. Default true. */
  rejectErrorDiagram?: boolean;
}
type E2EMermaidConfig = MermaidConfig & E2EConfig;

interface CodeObject {
  code: string | string[];
  mermaid: E2EMermaidConfig;
}

export const utf8ToB64 = (str: string): string => {
  return Buffer.from(decodeURIComponent(encodeURIComponent(str))).toString('base64');
};

/** Keep screenshot names within filesystem limits (ENAMETOOLONG on long test titles). */
const shortenScreenshotName = (name: string, maxLen = 180): string => {
  const sanitized = name.replace(/\s+/g, '-');
  if (sanitized.length <= maxLen) {
    return sanitized;
  }
  const hash = createHash('sha1').update(sanitized).digest('hex').slice(0, 8);
  return `${sanitized.slice(0, maxLen - 9)}-${hash}`;
};

export const mermaidUrl = (
  graphStr: string | string[],
  options: E2EMermaidConfig,
  api: boolean
): string => {
  options.handDrawnSeed = 1;
  options.architecture = { seed: 1, ...(options.architecture ?? {}) };
  options.cynefin = { seed: 1, ...(options.cynefin ?? {}) };
  const codeObject: CodeObject = {
    code: graphStr,
    mermaid: options,
  };
  const objStr: string = JSON.stringify(codeObject);
  let url = `/e2e.html?graph=${utf8ToB64(objStr)}`;
  if (api && typeof graphStr === 'string') {
    url = `/xss.html?graph=${graphStr}`;
  }

  if (options.listUrl) {
    console.log(options.listId, ' ', url);
  }

  return url;
};

export const imgSnapshotTest = async (
  page: Page,
  testInfo: TestInfo,
  graphStr: string,
  _options: E2EMermaidConfig = {},
  api = false,
  validation?: any
): Promise<void> => {
  const options: E2EMermaidConfig = {
    ..._options,
    fontFamily: _options.fontFamily ?? 'courier',
    // @ts-ignore TODO: Fix type of fontSize
    fontSize: _options.fontSize ?? '16px',
    sequence: {
      ...(_options.sequence ?? {}),
      actorFontFamily: 'courier',
      noteFontFamily: _options.sequence?.noteFontFamily ?? 'courier',
      messageFontFamily: 'courier',
    },
  };

  const url: string = mermaidUrl(graphStr, options, api);
  await openURLAndVerifyRendering(page, testInfo, url, options, validation);
};

export const urlSnapshotTest = async (
  page: Page,
  testInfo: TestInfo,
  url: string,
  options: E2EMermaidConfig = {},
  _api = false,
  validation?: any
): Promise<void> => {
  await openURLAndVerifyRendering(page, testInfo, url, options, validation);
};

export const renderGraph = async (
  page: Page,
  testInfo: TestInfo,
  graphStr: string | string[],
  options: E2EMermaidConfig = {},
  api = false
): Promise<void> => {
  const url: string = mermaidUrl(graphStr, options, api);
  await openURLAndVerifyRendering(page, testInfo, url, options);
};

/** Root mermaid diagram SVG — excludes nested icon/asset SVGs (e.g. architecture services). */
export const diagramSvg = (page: Page) => page.locator('svg[aria-roledescription]');

/** Assert the page shows a real diagram, not mermaid's syntax-error fallback SVG. */
export const assertDiagramNotError = async (page: Page): Promise<void> => {
  const svg = diagramSvg(page).first();
  await expect(svg).not.toHaveAttribute('aria-roledescription', 'error');
  await expect(svg.locator('.error-icon')).toHaveCount(0);
};

export const openURLAndVerifyRendering = async (
  page: Page,
  testInfo: TestInfo,
  url: string,
  { screenshot = true, rejectErrorDiagram = true, ...options }: E2EMermaidConfig,
  validation?: any
): Promise<void> => {
  const name: string = shortenScreenshotName(options.name ?? testInfo.titlePath.join(' '));

  // Capture browser-side errors so a render failure reports the real cause
  // instead of a bare "svg not visible" timeout.
  const pageErrors: string[] = [];
  const consoleErrors: string[] = [];
  page.on('pageerror', (error) => pageErrors.push(`[pageerror] ${error.message}`));
  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      consoleErrors.push(`[console.error] ${msg.text()}`);
    }
  });

  await startCoverage(page);
  await page.goto(url);

  // Prefer the explicit rendered flag from viewer.js; fall back to SVG visibility.
  try {
    await page.waitForFunction(() => (window as any).rendered === true, undefined, {
      timeout: 5_000,
    });
  } catch {
    try {
      await diagramSvg(page).first().waitFor({ state: 'visible', timeout: 15_000 });
    } catch (error) {
      const details = [...pageErrors, ...consoleErrors];
      if (details.length > 0) {
        throw new Error(
          `Diagram did not render (window.rendered never set, no SVG). Browser errors:\n${details.join('\n')}`
        );
      }
      throw error;
    }
  }

  // xssAttack() throws after injecting #the-malware and still leaves an SVG, so a
  // catch-only check misses it. Do not fail on every pageerror: error-diagram
  // tests and "render after error" cases throw mermaid parse errors on purpose.
  const xssErrors = pageErrors.filter((msg) => msg.includes('XSS Succeeded'));
  if (xssErrors.length > 0) {
    throw new Error(`XSS payload executed during render:\n${xssErrors.join('\n')}`);
  }

  if (options.securityLevel === 'sandbox') {
    const iframe = page.locator('iframe');
    await expect(iframe).toBeVisible();
    if (validation) {
      await validation(iframe);
    }
  } else {
    const svg = diagramSvg(page).first();
    await expect(svg).toBeVisible();
    await expect(svg).not.toHaveAttribute('viewbox'); // cspell:ignore viewbox

    if (rejectErrorDiagram) {
      await assertDiagramNotError(page);
    }

    if (validation) {
      await validation(svg);
    }
  }

  if (screenshot) {
    await verifyScreenshot(page, testInfo, name);
  }

  await collectCoverage(page, testInfo);
};

export const verifyScreenshot = async (
  page: Page,
  testInfo: TestInfo,
  name: string
): Promise<void> => {
  // Capture only the rendered diagram SVG so each screenshot is a tight crop of
  // the diagram rather than the whole viewport (mostly empty space). Fall back
  // to the full page when there is no top-level diagram SVG — e.g. sandboxed
  // diagrams (the SVG lives in an iframe) and the iife/xss tests.
  const svg = diagramSvg(page).first();
  const hasSvg = (await svg.count()) > 0;
  const target = hasSvg ? svg : page;

  const snapshotName = `${name}.png`;
  const snapshotPath = testInfo.snapshotPath(snapshotName, { kind: 'screenshot' });

  if (!existsSync(snapshotPath)) {
    mkdirSync(dirname(snapshotPath), { recursive: true });
    const screenshot = await target.screenshot({ animations: 'disabled', scale: 'device' });
    writeFileSync(snapshotPath, screenshot);
    return;
  }

  await expect(target).toHaveScreenshot(snapshotName);
};

/**
 * Asserts that no element ID appears more than once in the current document.
 */
export const assertNoDuplicateIds = async (page: Page): Promise<void> => {
  const duplicates = await page.evaluate(() => {
    const allElements = document.querySelectorAll('[id]');
    const idCounts: Record<string, number> = {};
    for (const el of allElements) {
      const id = el.getAttribute('id')!;
      idCounts[id] = (idCounts[id] || 0) + 1;
    }
    return Object.entries(idCounts).filter(([, count]) => count > 1);
  });

  expect(
    duplicates,
    `Duplicate IDs found: ${duplicates.map(([id, n]) => `${id} (${n}x)`).join(', ')}`
  ).toHaveLength(0);
};

export const verifyNumber = (value: number, expected: number, deltaPercent = 10): void => {
  const low = expected * (1 - deltaPercent / 100);
  const high = expected * (1 + deltaPercent / 100);
  expect(value).toBeGreaterThanOrEqual(low);
  expect(value).toBeLessThanOrEqual(high);
};
