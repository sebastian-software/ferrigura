import assert from 'node:assert/strict';
import { join } from 'node:path';

export async function checkLegendTiming(page, sample) {
  if (!['pie', 'donut', 'xychart', 'xy-horizontal'].includes(sample.id)) return;
  const plots = [...sample.source.matchAll(/^\s*(bar|line)(?:\s+"([^"]+)")?\s*\[/gm)].map(
    ([, kind, title], index) => ({ kind, title, index })
  );
  const timings = await page.evaluate(
    ({ pie, plots }) => {
      const delay = (element) => Number.parseFloat(getComputedStyle(element).animationDelay);
      const markDelay = (mark) => {
        const id = /#([^"')]+)/.exec(getComputedStyle(mark).maskImage)[1];
        return delay(document.getElementById(id).querySelector('[data-draw-part="mask"]'));
      };
      const expected = [];
      if (pie) {
        const sectors = [...document.querySelectorAll('.pieCircle')];
        document.querySelectorAll('[data-draw-part="legend"]').forEach((legend, index) => {
          expected.push([legend, sectors[index].closest('[data-draw-part="edge"]')]);
        });
      } else {
        const named = plots.filter((plot) => plot.title);
        const labels = [...document.querySelectorAll('.legend .label [data-draw-part="legend"]')];
        const markers = [
          ...document.querySelectorAll('.legend .markers [data-draw-part="legend"]'),
        ];
        for (const [index, legend] of labels.entries()) {
          const plot = named[index];
          expected.push([
            legend,
            document.querySelector(`.${plot.kind}-plot-${plot.index} [data-draw-part="edge"]`),
          ]);
        }
        for (const kind of ['bar', 'line']) {
          const kindPlots = named.filter((plot) => plot.kind === kind);
          const kindMarkers = markers.filter((marker) =>
            marker.querySelector(kind === 'bar' ? 'rect' : 'path')
          );
          for (const [index, marker] of kindMarkers.entries()) {
            const plot = kindPlots[index];
            expected.push([
              marker,
              document.querySelector(`.${kind}-plot-${plot.index} [data-draw-part="edge"]`),
            ]);
          }
        }
      }
      return {
        count: expected.length,
        entries: expected.map(([legend, mark]) => {
          const animation = legend.getAnimations()[0];
          const { delay: start, duration } = animation.effect.getTiming();
          const hidden = getComputedStyle(legend).opacity;
          animation.currentTime = start + duration / 2;
          const entering = Number(getComputedStyle(legend).opacity);
          animation.currentTime = start + duration + 1;
          const shown = getComputedStyle(legend).opacity;
          animation.currentTime = 0;
          return { actual: delay(legend), expected: markDelay(mark), hidden, entering, shown };
        }),
      };
    },
    { pie: ['pie', 'donut'].includes(sample.id), plots }
  );
  assert(timings.count > 0, `Missing animated legend for ${sample.id}`);
  for (const timing of timings.entries) {
    assert.equal(timing.actual, timing.expected, 'A legend entry must start with its own data');
    assert.equal(timing.hidden, '0');
    assert(timing.entering > 0 && timing.entering < 1);
    assert.equal(timing.shown, '1');
  }
}

export async function checkPieSeparators(context, origin, samples, output) {
  const page = await context.newPage();
  try {
    for (const sample of samples.filter((sample) => ['pie', 'donut'].includes(sample.id))) {
      const response = await page.request.get(
        `${origin}/render?sample=${sample.id}&order=source&speed=1`
      );
      assert(response.ok());
      await page.setContent(await response.text());
      await page.evaluate(() => {
        const svg = document.querySelector('svg');
        const box = svg.viewBox.baseVal;
        svg.style.cssText = `position:fixed;left:0;top:0;width:${box.width}px;height:${box.height}px;max-width:none;background:white`;
        svg.getAnimations({ subtree: true }).forEach((animation) => {
          animation.pause();
          animation.currentTime = 0;
        });
        // Isolate a completed first sector while the following sectors are still hidden.
        const edge = document.querySelector('.pieCircle').closest('[data-draw-part="edge"]');
        const mask = getComputedStyle(edge).maskImage;
        edge.style.animation = 'none';
        edge.style.maskImage = mask;
        document.querySelector('[data-draw-part="mask"]').style.cssText =
          'animation:none;stroke-dashoffset:0';
      });
      await page.evaluate(() => document.fonts.ready);
      const masked = await page
        .locator('svg')
        .screenshot({ path: join(output, `${sample.id}-separator-masked.png`) });
      await page
        .locator('.pieCircle')
        .first()
        .evaluate((path) => {
          path.closest('[data-draw-part="edge"]').style.maskImage = 'none';
        });
      const released = await page
        .locator('svg')
        .screenshot({ path: join(output, `${sample.id}-separator-released.png`) });
      const maxDelta = await page.evaluate(
        async (images) => {
          const pixels = await Promise.all(
            images.map(async (source) => {
              const image = new Image();
              image.src = `data:image/png;base64,${source}`;
              await image.decode();
              const canvas = document.createElement('canvas');
              canvas.width = image.width;
              canvas.height = image.height;
              const context = canvas.getContext('2d');
              context.drawImage(image, 0, 0);
              return context.getImageData(0, 0, image.width, image.height).data;
            })
          );
          return pixels[0].reduce(
            (delta, value, index) => Math.max(delta, Math.abs(value - pixels[1][index])),
            0
          );
        },
        [masked.toString('base64'), released.toString('base64')]
      );
      // Mask compositing can round a color channel by one. Clipping half a separator
      // produces a jump of up to 180, even though the final static SVG is correct.
      assert(
        maxDelta <= 1,
        `${sample.id}: removing the reveal mask changed its borders (${maxDelta}/255)`
      );
      console.log(
        `Verified ${sample.id}: separator thickness stays stable when the mask releases.`
      );
    }
  } finally {
    await page.close();
  }
}
