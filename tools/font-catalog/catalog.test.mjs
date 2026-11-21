import { test } from 'node:test';
import assert from 'node:assert/strict';
import { hash, prepare, readCatalog, uploadImmutable, verifyPublic } from './catalog.mjs';

const payload = Buffer.from('font bytes');
const path = `fonts/lexica-ultralegible/1.0.0/${hash(payload)}.woff2`;
const objects = new Map([[path, payload]]);
const config = { hostname: 'storage.bunnycdn.com', zone: 'test-zone', accessKey: 'test', objects };

test('catalog paths encode the full pinned hash', async () => {
  const { assets, manifestPath, bytes } = await readCatalog();
  assert.equal(assets.length, 9);
  assert(manifestPath.includes(hash(bytes)));
  for (const asset of assets) assert(asset.path.includes(asset.sha256));
});

test('preparation refuses source bytes that do not match the pinned hash', async () => {
  await assert.rejects(
    prepare('/unused', async () => new Response('wrong')),
    /integrity mismatch/
  );
});

test('upload reads back new objects and rejects a collision', async () => {
  let stored;
  const fetchImpl = async (_url, options) => {
    if (options.method === 'PUT') {
      stored = Buffer.from(options.body);
      return new Response(null, { status: 201 });
    }
    return stored ? new Response(stored) : new Response(null, { status: 404 });
  };
  await uploadImmutable({ ...config, fetchImpl });
  assert.deepEqual(stored, payload);
  stored = Buffer.from('other');
  await assert.rejects(uploadImmutable({ ...config, fetchImpl }), /differs/);
});

test('public verification requires bytes, CORS, MIME and immutable caching', async () => {
  const headers = {
    'access-control-allow-origin': '*',
    'content-type': 'font/woff2',
    'cache-control': 'public, max-age=31536000, immutable',
  };
  const fetchImpl = async () => new Response(payload, { headers });
  await verifyPublic('https://assets.example.org', objects, fetchImpl);
  await assert.rejects(
    verifyPublic(
      'https://assets.example.org',
      objects,
      async () =>
        new Response(payload, { headers: { ...headers, 'access-control-allow-origin': 'none' } })
    ),
    /CORS/
  );
  await assert.rejects(
    verifyPublic(
      'https://assets.example.org',
      objects,
      async () => new Response(Buffer.from('bad'), { headers })
    ),
    /integrity/
  );
  await assert.rejects(
    verifyPublic(
      'https://assets.example.org',
      objects,
      async () =>
        new Response(payload, {
          headers: { ...headers, 'content-type': 'application/octet-stream' },
        })
    ),
    /content type/
  );
  await assert.rejects(
    verifyPublic(
      'https://assets.example.org',
      objects,
      async () => new Response(payload, { headers: { ...headers, 'cache-control': 'max-age=600' } })
    ),
    /cache/
  );
});
