import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  hash,
  prepare,
  readCatalog,
  sharedStoragePassword,
  uploadImmutable,
  verifyPublic,
} from './catalog.mjs';

const payload = Buffer.from('font bytes');
const path = `fonts/lexica-ultralegible/1.0.0/${hash(payload)}.woff2`;
const objects = new Map([[path, payload]]);
const config = { accessKey: 'test', objects };

test('management lookup is read-only and requires the expected pull zone, hostname and storage', async () => {
  const calls = [];
  const responses = {
    '/pullzone?perPage=1000&page=1': [{ Name: 'sebastian-websites-assets', Id: 12 }],
    '/pullzone/12': {
      Id: 12,
      Name: 'sebastian-websites-assets',
      StorageZoneId: 34,
      Hostnames: [{ Value: 'assets.sebastian-software.com' }],
    },
    '/storagezone/34': { Id: 34, Name: 'sebastian-websites-assets', Password: 'test-password' },
  };
  const fetchImpl = async (url, options) => {
    calls.push([new URL(url).pathname + new URL(url).search, options.method]);
    return Response.json(responses[calls.at(-1)[0]]);
  };
  assert.equal(await sharedStoragePassword('test-key', fetchImpl), 'test-password');
  assert.deepEqual(
    calls.map((entry) => entry[1]),
    ['GET', 'GET', 'GET']
  );
  responses['/storagezone/34'].Name = 'other-zone';
  await assert.rejects(sharedStoragePassword('test-key', fetchImpl), /does not match/);
  responses['/storagezone/34'].Name = 'sebastian-websites-assets';
  responses['/pullzone/12'].Hostnames = [{ Value: 'other.example' }];
  await assert.rejects(sharedStoragePassword('test-key', fetchImpl), /unexpected/);
  responses['/pullzone/12'].Hostnames = [{ Value: 'assets.sebastian-software.com' }];
  responses['/pullzone?perPage=1000&page=1'].push({ Name: 'sebastian-websites-assets', Id: 13 });
  await assert.rejects(sharedStoragePassword('test-key', fetchImpl), /not uniquely found/);
  assert(calls.every((entry) => entry[1] === 'GET'));
});

test('management errors never include the response body', async () => {
  await assert.rejects(
    sharedStoragePassword('test-key', async () => new Response('secret-test-value')),
    (error) => {
      assert.equal(error.message, 'Invalid Bunny management response');
      return true;
    }
  );
});

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
  const fetchImpl = async (url, options) => {
    assert.equal(url, `https://storage.bunnycdn.com/sebastian-websites-assets/ferrigura/${path}`);
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

test('host-only TTFs require integrity and MIME but not browser headers', async () => {
  const ttfPath = `fonts/lexica-ultralegible/1.0.0/${hash(payload)}.ttf`;
  await verifyPublic(
    'https://assets.example.org',
    new Map([[ttfPath, payload]]),
    async () => new Response(payload, { headers: { 'content-type': 'font/ttf' } })
  );
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
