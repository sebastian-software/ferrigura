import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { resolveCatalogFonts } from '../font-catalog.mjs';

const ISSUER = 'https://downloads.ferrigura.dev/v1/downloads';
const PROTECTED = 'https://protected-assets.ferrigura.dev';
const DEFAULT_HASH = '33a0a2fac7f3c0f43e2b38ef79d482e3cc4c75fc3730e241c28a73bef79eeaa8';
const TOKEN = 'test-only-bearer';
const SIGNATURE = 'test-only-signed-token';
const sha = (bytes) => createHash('sha256').update(bytes).digest('hex');

async function setup(t) {
  const cacheDir = await mkdtemp(join(tmpdir(), 'ferrigura-protected-'));
  const originalFetch = globalThis.fetch;
  const hadToken = Object.hasOwn(process.env, 'FERRIGURA_DOWNLOAD_TOKEN');
  const previousToken = process.env.FERRIGURA_DOWNLOAD_TOKEN;
  t.after(async () => {
    globalThis.fetch = originalFetch;
    if (hadToken) process.env.FERRIGURA_DOWNLOAD_TOKEN = previousToken;
    else delete process.env.FERRIGURA_DOWNLOAD_TOKEN;
    await rm(cacheDir, { recursive: true, force: true });
  });
  const manifest = await readFile(new URL('./fixtures/catalog-default.json', import.meta.url));
  assert.equal(sha(manifest), DEFAULT_HASH);
  const catalog = JSON.parse(manifest);
  const files = [
    'LexicaUltralegible-Regular.ttf',
    'LexicaUltralegible-Italic.ttf',
    'LexicaUltralegible-Bold.ttf',
    'LexicaUltralegible-BoldItalic.ttf',
  ];
  const objects = new Map();
  const manifestPath = `/ferrigura/catalogs/fonts/1.0.0/${DEFAULT_HASH}.json`;
  objects.set(manifestPath, manifest);
  for (let i = 0; i < files.length; i++) {
    const bytes = await readFile(
      new URL(`../../../crates/ferrigura-text/fonts/${files[i]}`, import.meta.url)
    );
    objects.set(`/ferrigura/${catalog.families[0].faces[i].renderer.path}`, bytes);
  }
  const options = { cacheDir, families: ['Lexica Ultralegible'], protectedDownloads: true };
  return { cacheDir, manifest, manifestPath, objects, options };
}

function signed(path) {
  return `${PROTECTED}${path}?token=${SIGNATURE}&expires=1999999999`;
}

test('protected cold fetch uses issuer once per pinned object; warm and offline need no token', async (t) => {
  const f = await setup(t);
  process.env.FERRIGURA_DOWNLOAD_TOKEN = TOKEN;
  const calls = [];
  globalThis.fetch = async (url, options) => {
    const target = new URL(url);
    calls.push(target.origin);
    assert.equal(options.redirect, 'manual');
    if (target.href === ISSUER) {
      assert.equal(options.method, 'POST');
      assert.equal(options.headers.authorization, `Bearer ${TOKEN}`);
      const request = JSON.parse(options.body);
      const bytes = f.objects.get(request.path);
      assert.ok(bytes);
      assert.equal(request.sha256, sha(bytes));
      return Response.json({
        url: signed(request.path),
        bytes: bytes.length,
        sha256: request.sha256,
      });
    }
    assert.equal(target.origin, PROTECTED);
    assert.equal(options.headers, undefined, 'bearer token is never sent to the CDN');
    assert.equal(target.searchParams.get('token'), SIGNATURE);
    const bytes = f.objects.get(target.pathname);
    assert.ok(bytes);
    return new Response(bytes);
  };
  const cold = await resolveCatalogFonts(f.options);
  assert.equal(cold.fonts.length, 4);
  assert.deepEqual(cold.fallbacks, []);
  assert.equal(calls.filter((origin) => origin === 'https://downloads.ferrigura.dev').length, 5);
  assert.equal(calls.filter((origin) => origin === PROTECTED).length, 5);
  delete process.env.FERRIGURA_DOWNLOAD_TOKEN;
  globalThis.fetch = async () => {
    throw new Error('network should not be used');
  };
  assert.deepEqual(await resolveCatalogFonts(f.options), cold);
  assert.deepEqual(await resolveCatalogFonts({ ...f.options, offline: true }), cold);
});

test('protected cold or offline misses do not fall back to public downloads', async (t) => {
  const f = await setup(t);
  delete process.env.FERRIGURA_DOWNLOAD_TOKEN;
  let calls = 0;
  globalThis.fetch = async () => {
    calls++;
    throw new Error('must not fetch');
  };
  await assert.rejects(resolveCatalogFonts(f.options), /FERRIGURA_DOWNLOAD_TOKEN/);
  await assert.rejects(resolveCatalogFonts({ ...f.options, offline: true }), /offline cache miss/);
  assert.equal(calls, 0);
});

test('rejects invalid issuer data without requesting or revealing signed URLs', async (t) => {
  const f = await setup(t);
  process.env.FERRIGURA_DOWNLOAD_TOKEN = TOKEN;
  const cases = [
    (r) => {
      r.sha256 = '0'.repeat(64);
    },
    (r) => {
      r.bytes = 0;
    },
    (r) => {
      r.bytes = 2 * 1024 * 1024 + 1;
    },
    (r) => {
      r.url = r.url.replace(PROTECTED, 'https://other.example');
    },
    (r) => {
      r.url = r.url.replace('/ferrigura/catalogs/', '/ferrigura/fonts/');
    },
    (r) => {
      r.url += '&extra=x';
    },
    (r) => {
      r.url += '&token=duplicate';
    },
    (r) => {
      r.url = r.url.replace('expires=1999999999', 'expires=no');
    },
    (r) => {
      r.url += '#fragment';
    },
    (r) => {
      r.url = r.url.replace('https://', 'http://');
    },
    (r) => {
      r.url = r.url.replace('https://', 'https://user:pass@');
    },
  ];
  for (const mutate of cases) {
    let calls = 0;
    globalThis.fetch = async (url, options) => {
      calls++;
      assert.equal(String(url), ISSUER);
      const { path, sha256 } = JSON.parse(options.body);
      const result = { url: signed(path), bytes: f.manifest.length, sha256 };
      mutate(result);
      return Response.json(result);
    };
    await assert.rejects(resolveCatalogFonts(f.options), (error) => {
      assert.match(error.message, /invalid protected download/);
      assert.ok(!String(error.stack).includes(TOKEN));
      assert.ok(!String(error.stack).includes(SIGNATURE));
      return true;
    });
    assert.equal(calls, 1);
  }
});

test('issuer and CDN errors redact tokens and never retry against public origin', async (t) => {
  const f = await setup(t);
  process.env.FERRIGURA_DOWNLOAD_TOKEN = TOKEN;
  let calls = 0;
  globalThis.fetch = async () => {
    calls++;
    throw new Error(`leaked ${TOKEN} ${SIGNATURE}`);
  };
  await assert.rejects(
    resolveCatalogFonts(f.options),
    (error) => !String(error.stack).includes(TOKEN) && !String(error.stack).includes(SIGNATURE)
  );
  assert.equal(calls, 1);
  calls = 0;
  globalThis.fetch = async (url, options) => {
    calls++;
    if (String(url) === ISSUER) {
      const { path, sha256 } = JSON.parse(options.body);
      return Response.json({ url: signed(path), bytes: f.manifest.length, sha256 });
    }
    throw new Error(`request ${url} failed with ${TOKEN}`);
  };
  await assert.rejects(
    resolveCatalogFonts(f.options),
    (error) => !String(error.stack).includes(TOKEN) && !String(error.stack).includes(SIGNATURE)
  );
  assert.equal(calls, 2);
});

test('issuer size is enforced for manifest and face bytes', async (t) => {
  const f = await setup(t);
  process.env.FERRIGURA_DOWNLOAD_TOKEN = TOKEN;
  globalThis.fetch = async (url, options) => {
    if (String(url) === ISSUER) {
      const { path, sha256 } = JSON.parse(options.body);
      return Response.json({ url: signed(path), bytes: f.manifest.length - 1, sha256 });
    }
    return new Response(f.manifest);
  };
  await assert.rejects(resolveCatalogFonts(f.options), /too large|size or SHA-256/);
});

test('custom catalogs never receive the protected bearer token', async (t) => {
  const f = await setup(t);
  process.env.FERRIGURA_DOWNLOAD_TOKEN = TOKEN;
  const bytes = Buffer.from('custom font');
  const manifest = Buffer.from(
    JSON.stringify({
      schemaVersion: 1,
      families: [
        {
          family: 'Custom',
          version: '1',
          faces: [
            {
              weight: 400,
              style: 'normal',
              renderer: { path: `fonts/${sha(bytes)}.ttf`, sha256: sha(bytes), size: bytes.length },
            },
          ],
        },
      ],
    })
  );
  const catalog = {
    manifestUrl: 'https://mirror.example/catalog.json',
    manifestSha256: sha(manifest),
    assetBase: 'https://mirror.example/',
  };
  let calls = 0;
  globalThis.fetch = async (url, options) => {
    calls++;
    assert.equal(new URL(url).origin, 'https://mirror.example');
    assert.equal(options.headers, undefined);
    return new Response(new URL(url).pathname === '/catalog.json' ? manifest : bytes);
  };
  const result = await resolveCatalogFonts({ cacheDir: f.cacheDir, families: ['Custom'], catalog });
  assert.equal(result.fonts.length, 1);
  assert.equal(calls, 2);
  await assert.rejects(
    resolveCatalogFonts({
      cacheDir: f.cacheDir,
      families: ['Custom'],
      catalog,
      protectedDownloads: true,
    }),
    /built-in catalog/
  );
  assert.equal(calls, 2);
});

test('rejects redirects from issuer and CDN without retrying or exposing tokens', async (t) => {
  const f = await setup(t);
  process.env.FERRIGURA_DOWNLOAD_TOKEN = TOKEN;
  let calls = 0;
  globalThis.fetch = async () => {
    calls++;
    return new Response('issuer body', {
      status: 302,
      headers: { location: 'https://untrusted.test/' },
    });
  };
  await assert.rejects(resolveCatalogFonts(f.options), (error) => {
    assert.match(error.message, /issuer returned HTTP 302/);
    assert.ok(!String(error.stack).includes(TOKEN));
    return true;
  });
  assert.equal(calls, 1);
  calls = 0;
  globalThis.fetch = async (url, options) => {
    calls++;
    if (String(url) === ISSUER) {
      const { path, sha256 } = JSON.parse(options.body);
      return Response.json({ url: signed(path), bytes: f.manifest.length, sha256 });
    }
    return new Response('cdn body', {
      status: 302,
      headers: { location: 'https://untrusted.test/' },
    });
  };
  await assert.rejects(resolveCatalogFonts(f.options), (error) => {
    assert.match(error.message, /HTTP 302/);
    assert.ok(!String(error.stack).includes(SIGNATURE));
    return true;
  });
  assert.equal(calls, 2);
});

test('rejects mismatched signed face size and content before caching', async (t) => {
  const f = await setup(t);
  process.env.FERRIGURA_DOWNLOAD_TOKEN = TOKEN;
  let faceContentRequests = 0;
  globalThis.fetch = async (url, options) => {
    if (String(url) === ISSUER) {
      const { path, sha256 } = JSON.parse(options.body);
      const bytes = f.objects.get(path);
      return Response.json({
        url: signed(path),
        bytes: path === f.manifestPath ? bytes.length : bytes.length + 1,
        sha256,
      });
    }
    const path = new URL(url).pathname;
    if (path !== f.manifestPath) faceContentRequests++;
    return new Response(f.objects.get(path));
  };
  await assert.rejects(
    resolveCatalogFonts(f.options),
    /invalid protected download issuer response/
  );
  assert.equal(faceContentRequests, 0, 'wrong issuer size must fail before the face fetch');

  globalThis.fetch = async (url, options) => {
    if (String(url) === ISSUER) {
      const { path, sha256 } = JSON.parse(options.body);
      return Response.json({ url: signed(path), bytes: f.objects.get(path).length, sha256 });
    }
    const bytes = f.objects.get(new URL(url).pathname);
    const corrupt = Buffer.from(bytes);
    corrupt[0] ^= 1;
    return new Response(corrupt);
  };
  await assert.rejects(resolveCatalogFonts(f.options), /size or SHA-256/);
});
