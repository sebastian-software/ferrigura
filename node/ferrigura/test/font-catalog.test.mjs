import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, rm, symlink, unlink, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { gzipSync } from 'node:zlib';
import { resolveCatalogFonts } from '../font-catalog.mjs';

const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const face = (bytes, weight = 400, style = 'normal') => ({
  weight,
  style,
  renderer: { path: `fonts/${hash(bytes)}.ttf`, sha256: hash(bytes), size: bytes.length },
});
async function fixture(t, families) {
  const directory = await mkdtemp(join(tmpdir(), 'ferrigura-catalog-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const calls = [];
  const routes = new Map();
  const server = createServer((req, res) => {
    calls.push(req.url);
    const entry = routes.get(req.url);
    if (!entry) {
      res.writeHead(404).end();
      return;
    }
    if (entry.redirect) {
      res.writeHead(302, { location: entry.redirect }).end();
      return;
    }
    if (entry.compressed) {
      const compressed = gzipSync(entry.bytes);
      res
        .writeHead(200, { 'content-length': compressed.length, 'content-encoding': 'gzip' })
        .end(compressed);
      return;
    }
    res.writeHead(200, { 'content-length': entry.bytes.length }).end(entry.bytes);
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  t.after(() => new Promise((resolve) => server.close(resolve)));
  const origin = `http://127.0.0.1:${server.address().port}`;
  const catalog = { schemaVersion: 1, families };
  const manifest = Buffer.from(JSON.stringify(catalog));
  routes.set('/manifest.json', { bytes: manifest });
  return {
    directory,
    server,
    calls,
    routes,
    manifest,
    catalog: {
      manifestUrl: `${origin}/manifest.json`,
      manifestSha256: hash(manifest),
      assetBase: `${origin}/assets/`,
    },
  };
}

test('verified catalog faces become explicit aliases and work offline from cache', async (t) => {
  const regular = Buffer.from('primary regular');
  const bold = Buffer.from('primary bold');
  const fallback = Buffer.from('fallback font');
  const families = [
    { family: 'Primary', version: '1', faces: [face(regular), face(bold, 700)] },
    { family: 'Fallback', version: '1', faces: [face(fallback)] },
  ];
  const f = await fixture(t, families);
  for (const bytes of [regular, bold, fallback])
    f.routes.set(`/assets/fonts/${hash(bytes)}.ttf`, { bytes });
  const opts = {
    cacheDir: f.directory,
    catalog: f.catalog,
    families: [{ family: 'primary', alias: 'My Primary' }, 'Fallback'],
    fallbacks: ['Fallback'],
  };
  const result = await resolveCatalogFonts(opts);
  assert.deepEqual(
    result.fonts.map(({ family, weight, style, data }) => [
      family,
      weight,
      style,
      Buffer.from(data).toString(),
    ]),
    [
      ['My Primary', 400, 'normal', 'primary regular'],
      ['My Primary', 700, 'normal', 'primary bold'],
      ['Fallback', 400, 'normal', 'fallback font'],
    ]
  );
  assert.deepEqual(result.fallbacks, ['Fallback']);
  assert.equal(f.calls.length, 4);
  const noFallback = await resolveCatalogFonts({ ...opts, fallbacks: undefined });
  assert.deepEqual(noFallback.fallbacks, []);
  await assert.rejects(
    resolveCatalogFonts({ ...opts, fallbacks: ['Missing'] }),
    /not a selected alias/
  );
  const offline = await resolveCatalogFonts({ ...opts, offline: true });
  assert.deepEqual(offline, result);
  assert.equal(f.calls.length, 4);
  await writeFile(join(f.directory, 'sha256', hash(regular)), 'corrupt');
  await assert.rejects(
    resolveCatalogFonts({ ...opts, offline: true }),
    /cached object .*wrong (size|SHA-256)/
  );
  await assert.rejects(resolveCatalogFonts(opts), /cached object .*wrong (size|SHA-256)/);
});

test('rejects wrong pins, offline misses, redirects and off-origin URLs', async (t) => {
  const bytes = Buffer.from('font bytes');
  const f = await fixture(t, [{ family: 'One', version: '1', faces: [face(bytes)] }]);
  const opts = { cacheDir: f.directory, families: ['One'], catalog: f.catalog };
  await assert.rejects(
    resolveCatalogFonts({ ...opts, catalog: { ...f.catalog, manifestSha256: '0'.repeat(64) } }),
    /SHA-256/
  );
  await assert.rejects(resolveCatalogFonts({ ...opts, offline: true }), /offline cache miss/);
  await assert.rejects(
    resolveCatalogFonts({
      ...opts,
      catalog: { ...f.catalog, assetBase: 'https://elsewhere.test/fonts/' },
    }),
    /allowed origin/
  );
  await assert.rejects(
    resolveCatalogFonts({
      ...opts,
      catalog: { ...f.catalog, manifestUrl: `${f.catalog.manifestUrl}?token=secret` },
    }),
    (error) => /manifestUrl/.test(error.message) && !error.message.includes('secret')
  );
  f.routes.set('/manifest.json', { redirect: 'https://elsewhere.test/redirect' });
  await assert.rejects(resolveCatalogFonts(opts), /HTTP 302/);
});

test('rejects malformed catalogs, paths, aliases and tampered face bytes', async (t) => {
  const bytes = Buffer.from('font bytes');
  const f = await fixture(t, [{ family: 'One', version: '1', faces: [face(bytes)] }]);
  const opts = { cacheDir: f.directory, families: ['One'], catalog: f.catalog };
  f.routes.set(`/assets/fonts/${hash(bytes)}.ttf`, { bytes: Buffer.from('other byte') });
  await assert.rejects(resolveCatalogFonts(opts), /size|SHA-256/);
  await assert.rejects(
    resolveCatalogFonts({ ...opts, families: ['One', { family: 'One', alias: 'ONE' }] }),
    /duplicate font alias/
  );
  await assert.rejects(
    resolveCatalogFonts({ ...opts, families: ['Missing'] }),
    /not in the catalog/
  );
  for (const mutate of [
    (m) => {
      m.schemaVersion = 2;
    },
    (m) => {
      m.families[0].version = '';
    },
    (m) => {
      m.families[0].faces[0].renderer.path = 'fonts/other.ttf';
    },
    (m) => {
      m.families[0].faces[0].renderer.path = '../escape.ttf';
    },
    (m) => {
      m.families[0].faces.push(m.families[0].faces[0]);
    },
    (m) => {
      m.families.push(m.families[0]);
    },
  ]) {
    const doc = structuredClone(JSON.parse(f.manifest));
    mutate(doc);
    const manifest = Buffer.from(JSON.stringify(doc));
    f.routes.set('/manifest.json', { bytes: manifest });
    await assert.rejects(
      resolveCatalogFonts({ ...opts, catalog: { ...f.catalog, manifestSha256: hash(manifest) } })
    );
  }
});

test('the built-in immutable catalog pin resolves Lexica faces', async (t) => {
  const directory = await mkdtemp(join(tmpdir(), 'ferrigura-default-catalog-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const manifest = await readFile(new URL('./fixtures/catalog-default.json', import.meta.url));
  const parsed = JSON.parse(manifest);
  const files = [
    'LexicaUltralegible-Regular.ttf',
    'LexicaUltralegible-Italic.ttf',
    'LexicaUltralegible-Bold.ttf',
    'LexicaUltralegible-BoldItalic.ttf',
  ];
  const routes = new Map();
  for (let i = 0; i < files.length; i++) {
    const bytes = await readFile(
      new URL(`../../../crates/ferrigura-text/fonts/${files[i]}`, import.meta.url)
    );
    routes.set(`/ferrigura/${parsed.families[0].faces[i].renderer.path}`, bytes);
  }
  const original = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = original;
  });
  globalThis.fetch = async (url) => {
    const target = new URL(url);
    assert.equal(target.origin, 'https://assets.sebastian-software.com');
    const bytes = target.pathname.includes('/catalogs/') ? manifest : routes.get(target.pathname);
    assert.ok(bytes, target.href);
    return new Response(bytes, { status: 200 });
  };
  const result = await resolveCatalogFonts({
    cacheDir: directory,
    families: ['Lexica Ultralegible'],
  });
  assert.deepEqual(
    result.fonts.map(({ weight, style }) => [weight, style]),
    [
      [400, 'normal'],
      [400, 'italic'],
      [700, 'normal'],
      [700, 'italic'],
    ]
  );
});

test('monospace aliases are independent of explicit fallbacks', async (t) => {
  const first = Buffer.from('primary');
  const code = Buffer.from('code');
  const f = await fixture(t, [
    { family: 'Main', version: '1', faces: [face(first)] },
    { family: 'Code', version: '1', faces: [face(code)] },
  ]);
  for (const bytes of [first, code]) f.routes.set(`/assets/fonts/${hash(bytes)}.ttf`, { bytes });
  const options = {
    cacheDir: f.directory,
    catalog: f.catalog,
    families: ['Main', { family: 'Code', alias: 'monospace' }],
  };
  const result = await resolveCatalogFonts(options);
  assert.deepEqual(
    result.fonts.map((font) => font.family),
    ['Main', 'monospace']
  );
  assert.deepEqual(result.fallbacks, []);
  assert.deepEqual(
    (await resolveCatalogFonts({ ...options, fallbacks: ['MONOSPACE'] })).fallbacks,
    ['monospace']
  );
});

test('rejects non-regular cache entries without following symlinks', async (t) => {
  const bytes = Buffer.from('font');
  const f = await fixture(t, [{ family: 'One', version: '1', faces: [face(bytes)] }]);
  const directory = join(f.directory, 'sha256');
  await mkdir(directory);
  const cached = join(directory, f.catalog.manifestSha256);
  await mkdir(cached);
  const options = { cacheDir: f.directory, catalog: f.catalog, families: ['One'], offline: true };
  await assert.rejects(resolveCatalogFonts(options), /not a regular file/);
  await rm(cached, { recursive: true });
  const other = join(f.directory, 'other');
  await writeFile(other, f.manifest);
  try {
    await symlink(other, cached);
  } catch (error) {
    if (process.platform === 'win32' && ['EPERM', 'EACCES'].includes(error.code)) {
      t.diagnostic('Symlink creation is unavailable; directory rejection was still checked');
      return;
    }
    throw error;
  }
  await assert.rejects(resolveCatalogFonts(options), /not a regular file|ELOOP/);
  await unlink(cached);
});

test('uses decoded byte size for compressed HTTP responses', async (t) => {
  const bytes = Buffer.from('font bytes repeated '.repeat(20));
  const f = await fixture(t, [{ family: 'One', version: '1', faces: [face(bytes)] }]);
  f.routes.set(`/assets/fonts/${hash(bytes)}.ttf`, { bytes, compressed: true });
  const result = await resolveCatalogFonts({
    cacheDir: f.directory,
    catalog: f.catalog,
    families: ['One'],
  });
  assert.deepEqual(result.fonts[0].data, bytes);
});

test('cancels an oversized streamed face response', async (t) => {
  const bytes = Buffer.from('font');
  const f = await fixture(t, [{ family: 'One', version: '1', faces: [face(bytes)] }]);
  const original = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = original;
  });
  let cancelled = false;
  globalThis.fetch = async (url, options) => {
    if (new URL(url).pathname === '/manifest.json') return original(url, options);
    return new Response(
      new ReadableStream({
        pull(controller) {
          controller.enqueue(new Uint8Array(bytes.length + 1));
        },
        cancel() {
          cancelled = true;
        },
      })
    );
  };
  await assert.rejects(
    resolveCatalogFonts({ cacheDir: f.directory, catalog: f.catalog, families: ['One'] }),
    /too large/
  );
  assert.equal(cancelled, true);
});

test('concurrent resolutions publish complete objects and leave no temporary files', async (t) => {
  const bytes = Buffer.from('shared font');
  const f = await fixture(t, [{ family: 'One', version: '1', faces: [face(bytes)] }]);
  f.routes.set(`/assets/fonts/${hash(bytes)}.ttf`, { bytes });
  const options = { cacheDir: f.directory, catalog: f.catalog, families: ['One'] };
  const outputs = await Promise.all(Array.from({ length: 5 }, () => resolveCatalogFonts(options)));
  for (const output of outputs) assert.deepEqual(output.fonts[0].data, bytes);
  const names = await (await import('node:fs/promises')).readdir(join(f.directory, 'sha256'));
  assert.deepEqual(names.sort(), [f.catalog.manifestSha256, hash(bytes)].sort());
});

test('cancels rejected HTTP response bodies and sets a timeout signal', async (t) => {
  const f = await fixture(t, [{ family: 'One', version: '1', faces: [face(Buffer.from('font'))] }]);
  const original = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = original;
  });
  let cancelled = false;
  globalThis.fetch = async (_url, options) => {
    assert.equal(options.redirect, 'manual');
    assert.ok(options.signal instanceof AbortSignal);
    return new Response(
      new ReadableStream({
        cancel() {
          cancelled = true;
        },
      }),
      { status: 302 }
    );
  };
  await assert.rejects(
    resolveCatalogFonts({ cacheDir: f.directory, catalog: f.catalog, families: ['One'] }),
    /HTTP 302/
  );
  assert.equal(cancelled, true);
});
