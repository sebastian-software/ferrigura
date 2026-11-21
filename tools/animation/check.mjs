import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { chromium } from '../port/node_modules/playwright/index.mjs';
import { checkLegendTiming, checkPieSeparators } from './chart-check.mjs';

async function geometry(page, markup) {
  return page.evaluate((markup) => {
    const svg =
      markup == null
        ? document.querySelector('#drawing > svg')
        : new DOMParser().parseFromString(markup, 'image/svg+xml').documentElement;
    return {
      viewBox: svg.getAttribute('viewBox'),
      marks: [...svg.querySelectorAll('path,rect,circle,ellipse,line,polygon,polyline,text,image')]
        .filter((element) => !element.closest('[data-draw-defs],[data-draw-part="marker"]'))
        .map((element) => [
          element.tagName,
          [...element.attributes]
            .filter((attribute) => !['marker-start', 'marker-end'].includes(attribute.name))
            .map((attribute) => [attribute.name, attribute.value]),
          element.tagName === 'text' ? element.textContent : '',
        ]),
    };
  }, markup);
}

const output = join(tmpdir(), 'ferrigura-animation');
await mkdir(output, { recursive: true });
const server = spawn(
  process.execPath,
  [fileURLToPath(new URL('./preview.mjs', import.meta.url)), '0'],
  {
    stdio: ['ignore', 'pipe', 'inherit'],
  }
);
let browser;
try {
  const origin = await new Promise((resolve, reject) => {
    server.stdout.on('data', (chunk) => {
      const url = /http:\/\/127\.0\.0\.1:\d+/.exec(String(chunk));
      if (url) resolve(url[0]);
    });
    server.once('error', reject);
    server.once('exit', (code) => reject(new Error(`Preview server exited with ${code}`)));
  });
  browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1280, height: 1000 },
    reducedMotion: 'no-preference',
  });
  const page = await context.newPage();
  const errors = [];
  const pixelFailures = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto(origin);
  await page.waitForFunction(() => !document.getElementById('download').disabled);
  const samples = (await (await page.request.get(`${origin}/samples.json`)).json()).samples;
  assert.equal(await page.locator('#order').inputValue(), 'source');

  for (const sample of samples) {
    await page.selectOption('#sample', sample.id);
    for (const order of ['flow', 'source']) {
      await page.selectOption('#order', order);
      await page.click('#finish');
      const velocities = await page.evaluate(() => {
        return [...document.querySelectorAll('[data-draw-part="mask"]')].map(
          (mask) =>
            Number(mask.getAttribute('data-draw-length')) /
            Number.parseFloat(getComputedStyle(mask).animationDuration)
        );
      });
      assert(
        // Millisecond CSS durations are rounded in the SVG serialization.
        velocities.every((velocity) => Math.abs(velocity - 700) < 0.7),
        `Average speed changed with path length or diagram size: ${velocities}`
      );
      await page.evaluate(() => {
        const input = document.getElementById('progress');
        input.value = '0';
        input.dispatchEvent(new Event('input', { bubbles: true }));
      });
      await checkLegendTiming(page, sample);
      const easedSections = await page.locator('[data-draw-part="mask"]').evaluateAll((masks) =>
        masks.slice(0, 2).map((mask) => {
          const animation = mask.getAnimations()[0];
          const { delay, duration } = animation.effect.getTiming();
          const offsets = [0.05, 0.5, 0.95].map((fraction) => {
            animation.currentTime = delay + duration * fraction;
            return Number.parseFloat(getComputedStyle(mask).strokeDashoffset);
          });
          return { easing: getComputedStyle(mask).animationTimingFunction, offsets };
        })
      );
      for (const {
        easing,
        offsets: [start, middle, end],
      } of easedSections) {
        assert.equal(easing, 'cubic-bezier(0.4, 0, 0.2, 1)');
        assert(start > 0.98 && start <= 1, 'Each section must accelerate gently from rest');
        assert(
          middle > 0.1 && middle < 0.4,
          'Easing must apply to each section, not the whole diagram'
        );
        assert(end >= 0 && end < 0.01, 'Each section must settle gently without overshooting');
      }
      await page.click('#finish');
      assert.equal(await page.locator('#progress').inputValue(), '1000');
      const originalGeometry = await geometry(page, sample.svg);
      const finalGeometry = await geometry(page);
      assert.deepEqual(
        finalGeometry,
        originalGeometry,
        'Animation changed original geometry or line styles'
      );
      await page.evaluate(() => {
        const input = document.getElementById('progress');
        input.value = '0';
        input.dispatchEvent(new Event('input', { bubbles: true }));
      });
      assert.equal(await page.locator('#progress').inputValue(), '0');
      assert.deepEqual(
        await page
          .locator('[data-draw-part="mask"]')
          .evaluateAll((paths) => paths.map((path) => getComputedStyle(path).strokeDashoffset)),
        Array(await page.locator('[data-draw-part="mask"]').count()).fill('1px')
      );
      assert.deepEqual(
        await page
          .locator('[data-draw-part="marker"]')
          .evaluateAll((arrows) => arrows.map((arrow) => getComputedStyle(arrow).opacity)),
        Array(await page.locator('[data-draw-part="marker"]').count()).fill('0')
      );
      await page.evaluate(() => {
        const input = document.getElementById('progress');
        input.value = '500';
        input.dispatchEvent(new Event('input', { bubbles: true }));
      });
      assert.equal(
        await page.locator('#progress').inputValue(),
        '500',
        'Scrubbing did not seek to the requested time'
      );
      await page.screenshot({ path: join(output, `${sample.id}-${order}-half.png`) });
      await page.click('#play');
      await page.waitForFunction(() => document.getElementById('play').textContent === 'Pause');
      await page.click('#play');
      const paused = await page.locator('#progress').inputValue();
      await page.waitForTimeout(100);
      assert.equal(
        await page.locator('#progress').inputValue(),
        paused,
        'Paused playback kept advancing'
      );
      await page.click('#finish');
      const markers = await page
        .locator('[data-draw-part="marker"]')
        .evaluateAll((arrows) => arrows.map((arrow) => getComputedStyle(arrow).opacity));
      assert(
        markers.every((opacity) => opacity === '1'),
        `Arrowheads missing at the end: ${markers}`
      );
      // Compare at unit scale and an integer origin, as the reference geometry harness does.
      // Fractional scaling in the demo can change SVG filter rasterization at group boundaries.
      const unitScale = (svg) => {
        const box = svg.viewBox.baseVal;
        svg.style.cssText = `position:fixed;left:0;top:0;width:${box.width}px;height:${box.height}px;max-width:none;pointer-events:none;background:white`;
      };
      await page.locator('#drawing > svg').evaluate(unitScale);
      const finalPixels = await page
        .locator('#drawing > svg')
        .screenshot({ path: join(output, 'completed.png') });
      await page.locator('#drawing > svg').evaluate((svg, markup) => {
        svg.replaceWith(new DOMParser().parseFromString(markup, 'image/svg+xml').documentElement);
      }, sample.svg);
      await page.locator('#drawing > svg').evaluate(unitScale);
      const originalPixels = await page
        .locator('#drawing > svg')
        .screenshot({ path: join(output, 'original.png') });
      const pixelDifference = await page.evaluate(
        async (images) => {
          let width;
          const pixels = await Promise.all(
            images.map(async (source) => {
              const image = new Image();
              image.src = `data:image/png;base64,${source}`;
              await image.decode();
              const canvas = document.createElement('canvas');
              canvas.width = image.width;
              width = image.width;
              canvas.height = image.height;
              const context = canvas.getContext('2d');
              context.drawImage(image, 0, 0);
              return context.getImageData(0, 0, image.width, image.height).data;
            })
          );
          let changed = 0;
          let maxDelta = 0;
          let minX = width,
            maxX = 0,
            minY = Infinity,
            maxY = 0;
          for (let i = 0; i < pixels[0].length; i++)
            if (pixels[0][i] !== pixels[1][i]) {
              changed++;
              maxDelta = Math.max(maxDelta, Math.abs(pixels[0][i] - pixels[1][i]));
              const pixel = Math.floor(i / 4),
                x = pixel % width,
                y = Math.floor(pixel / width);
              minX = Math.min(minX, x);
              maxX = Math.max(maxX, x);
              minY = Math.min(minY, y);
              maxY = Math.max(maxY, y);
            }
          return {
            changed,
            maxDelta,
            size: pixels[0].length,
            otherSize: pixels[1].length,
            bounds: [minX, minY, maxX, maxY],
          };
        },
        [finalPixels.toString('base64'), originalPixels.toString('base64')]
      );
      if (pixelDifference.changed)
        pixelFailures.push({ sample: sample.id, order, ...pixelDifference });
      console.log(`Verified ${sample.id}/${order}: geometry, masks, markers, scrub and pause.`);
    }
  }

  await checkPieSeparators(context, origin, samples, output);
  await page.selectOption('#sample', 'request');
  await page.selectOption('#order', 'source');
  await page.evaluate(() => {
    const input = document.getElementById('speed');
    input.value = '2';
    input.dispatchEvent(new Event('change', { bubbles: true }));
  });
  await page.waitForFunction(() => !document.getElementById('download').disabled);
  const fastVelocity = await page.evaluate(() => {
    const edge = document.querySelector('[data-edge]');
    const mask = document.querySelector('[data-draw-part="mask"]');
    return edge.getTotalLength() / Number.parseFloat(getComputedStyle(mask).animationDuration);
  });
  assert(Math.abs(fastVelocity - 1400) < 1.4, 'Speed multiplier did not double the line velocity');
  await page.click('#replay');
  await page.waitForFunction(
    () => document.getElementById('status').textContent.startsWith('Complete.'),
    undefined,
    { timeout: 7000 }
  );
  assert.equal(await page.locator('#progress').inputValue(), '1000');
  const completedMarkers = await page
    .locator('[data-draw-part="marker"]')
    .evaluateAll((arrows) => arrows.map((arrow) => getComputedStyle(arrow).opacity));
  assert(
    completedMarkers.every((opacity) => opacity === '1'),
    `Playback lost arrowheads: ${completedMarkers}`
  );

  const downloadPromise = page.waitForEvent('download');
  await page.click('#download');
  const download = await downloadPromise;
  const exportPath = join(output, download.suggestedFilename());
  await download.saveAs(exportPath);
  const exported = await readFile(exportPath, 'utf8');
  assert(!exported.includes('<script'), 'Export must not contain JavaScript');
  const standalone = await browser.newContext({
    javaScriptEnabled: false,
    reducedMotion: 'no-preference',
  });
  const exportPage = await standalone.newPage();
  for (const sample of samples) {
    const samplePath =
      sample.id === 'request' ? exportPath : join(output, `${sample.id}-export.svg`);
    if (sample.id !== 'request') {
      const response = await page.request.get(
        `${origin}/render?${new URLSearchParams({ sample: sample.id, speed: '2', order: 'source' })}`
      );
      assert(response.ok(), `Export render failed for ${sample.id}`);
      const markup = await response.text();
      assert(!markup.includes('<script'), 'Export must not contain JavaScript');
      await writeFile(samplePath, markup);
    }
    await exportPage.emulateMedia({ reducedMotion: 'no-preference' });
    await exportPage.goto(pathToFileURL(samplePath).href);
    const first = await exportPage.screenshot();
    const exportDuration = Number(
      await exportPage.locator('[data-draw-duration]').getAttribute('data-draw-duration')
    );
    await exportPage.waitForTimeout(Math.ceil(exportDuration + 100));
    const last = await exportPage.screenshot();
    await exportPage.screenshot({ path: join(output, 'export-last.png') });
    assert(!first.equals(last), 'Standalone SVG did not animate without JavaScript');
    assert(
      await exportPage
        .locator('[data-draw-part="mask"]')
        .evaluateAll((paths) =>
          paths.every((path) => getComputedStyle(path).strokeDashoffset === '0px')
        ),
      'Standalone masks did not finish'
    );
    assert(
      await exportPage
        .locator('[data-draw-part="marker"]')
        .evaluateAll((arrows) => arrows.every((arrow) => getComputedStyle(arrow).opacity === '1')),
      'Standalone arrowheads did not finish'
    );
    await exportPage.emulateMedia({ reducedMotion: 'reduce' });
    await exportPage.reload();
    assert.equal(
      await exportPage
        .locator('svg')
        .evaluate((svg) => svg.getAnimations({ subtree: true }).length),
      0
    );
    assert(
      await exportPage
        .locator('[data-draw-part="mask"]')
        .evaluateAll((paths) =>
          paths.every((path) => getComputedStyle(path).strokeDashoffset === '0px')
        )
    );
    assert(
      await exportPage
        .locator('[data-draw-part]:not([data-draw-part="mask"])')
        .evaluateAll((parts) => parts.every((part) => getComputedStyle(part).opacity === '1'))
    );
    console.log(`Verified ${sample.id}: standalone SVG without JavaScript and reduced motion.`);
  }
  await standalone.close();

  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForFunction(() => document.getElementById('play').disabled);
  assert.equal(await page.locator('#play').isDisabled(), true);
  assert.equal(
    await page
      .locator('#drawing > svg')
      .evaluate((svg) => svg.getAnimations({ subtree: true }).length),
    0
  );
  assert(
    await page
      .locator('[data-draw-part="node"]')
      .evaluateAll((nodes) => nodes.every((node) => getComputedStyle(node).opacity === '1'))
  );
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.waitForFunction(() => !document.getElementById('play').disabled);
  assert.equal(await page.locator('#play').isEnabled(), true);

  for (const width of [320, 390, 768, 1280]) {
    await page.setViewportSize({ width, height: 1000 });
    for (const sample of samples) {
      await page.selectOption('#sample', sample.id);
      await page.click('#finish');
      assert(
        await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
        `Horizontal overflow for ${sample.id} at ${width}px`
      );
      assert(
        await page.locator('#drawing > svg').evaluate((svg) => {
          const bounds = svg.getBoundingClientRect();
          const parent = svg.parentElement.getBoundingClientRect();
          return bounds.left >= parent.left - 1 && bounds.right <= parent.right + 1;
        }),
        `SVG exceeds its canvas for ${sample.id} at ${width}px`
      );
      await page.screenshot({
        path: join(output, `preview-${sample.id}-${width}.png`),
        fullPage: true,
      });
    }
  }
  assert.deepEqual(errors, [], 'Browser runtime errors');
  assert.deepEqual(pixelFailures, [], 'Completed SVG pixel differences');
  console.log(
    `Verified playback completion, script-free SVG export, reduced motion, and four viewport widths. Artifacts: ${output}`
  );
} finally {
  await browser?.close();
  server.kill();
}
