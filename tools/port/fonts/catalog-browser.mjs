import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { chromium } from 'playwright';
import { readCatalog } from '../../font-catalog/catalog.mjs';

const { family, assets } = await readCatalog();
const directory = resolve('target/font-catalog');
const allowed = new Set(assets.map((asset) => `/${asset.path}`));
const listen = (server) =>
  new Promise((done) => server.listen(0, '127.0.0.1', () => done(server.address().port)));
const pageServer = createServer((_request, response) =>
  response
    .writeHead(200, { 'content-type': 'text/html' })
    .end('<!doctype html><title>Font test</title>')
);
const assetServer = createServer(async (request, response) => {
  const url = new URL(request.url, 'http://localhost');
  if (!allowed.has(url.pathname)) {
    response.writeHead(404).end();
    return;
  }
  const bytes = await readFile(resolve(directory, url.pathname.slice(1)));
  const headers = {
    'content-type': url.pathname.endsWith('.woff2')
      ? 'font/woff2'
      : url.pathname.endsWith('.ttf')
        ? 'font/ttf'
        : 'text/plain',
  };
  if (!url.searchParams.has('deny')) headers['access-control-allow-origin'] = '*';
  response.writeHead(200, headers).end(bytes);
});
const [pagePort, assetPort] = await Promise.all([listen(pageServer), listen(assetServer)]);
const browser = await chromium.launch({ args: ['--font-render-hinting=none'] });
try {
  const page = await browser.newPage();
  await page.goto(`http://127.0.0.1:${pagePort}/`);
  const result = await page.evaluate(
    async ({ faces, origin }) => {
      const result = [];
      for (const face of faces) {
        const name = `${face.weight}-${face.style}`;
        const ttf = new FontFace(`ttf-${name}`, `url(${origin}/${face.renderer.path})`, {
          weight: String(face.weight),
          style: face.style,
        });
        const woff = new FontFace(`woff-${name}`, `url(${origin}/${face.web.path})`, {
          weight: String(face.weight),
          style: face.style,
        });
        await Promise.all([ttf.load(), woff.load()]);
        document.fonts.add(ttf);
        document.fonts.add(woff);
        const context = document.createElement('canvas').getContext('2d');
        const metrics = (font) => {
          context.font = `${face.style} ${face.weight} 32px "${font}"`;
          const box = context.measureText('Hamburgefontsiv AVATAR office italic αβγ ﬂ');
          return [
            box.width,
            box.actualBoundingBoxLeft,
            box.actualBoundingBoxRight,
            box.actualBoundingBoxAscent,
            box.actualBoundingBoxDescent,
          ];
        };
        result.push({
          name,
          ttf: metrics(`ttf-${name}`),
          woff: metrics(`woff-${name}`),
          loaded: document.fonts.check(`${face.style} ${face.weight} 32px "woff-${name}"`),
        });
      }
      let denied = false;
      try {
        await fetch(`${origin}/${faces[0].web.path}?deny`);
      } catch {
        denied = true;
      }
      return { result, denied };
    },
    { faces: family.faces, origin: `http://127.0.0.1:${assetPort}` }
  );
  assert(result.denied, 'A cross-origin request without CORS must fail');
  for (const face of result.result) {
    assert(face.loaded, face.name);
    assert.deepEqual(face.ttf, face.woff, face.name);
  }
  console.log(`Verified ${result.result.length} TTF/WOFF2 pairs and cross-origin loading`);
} finally {
  await browser.close();
  pageServer.close();
  assetServer.close();
}
