import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const catalogFile = fileURLToPath(new URL('./catalog.json', import.meta.url));
export const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const SOURCE = 'https://raw.githubusercontent.com/jacobxperez/lexica-ultralegible/';
const PATH = /^fonts\/lexica-ultralegible\/1\.0\.0\/[0-9a-f]{64}\.(?:ttf|woff2|txt)$/;
const SHA = /^[0-9a-f]{64}$/;

export async function readCatalog() {
  const bytes = await readFile(catalogFile);
  const catalog = JSON.parse(bytes);
  if (catalog.schemaVersion !== 1 || catalog.families?.length !== 1)
    throw new Error('Invalid catalog');
  const family = catalog.families[0];
  if (
    !/^[0-9a-f]{40}$/.test(family.sourceCommit) ||
    family.source !==
      `https://github.com/jacobxperez/lexica-ultralegible/tree/${family.sourceCommit}` ||
    family.family !== 'Lexica Ultralegible' ||
    family.version !== '1.0.0' ||
    family.license?.spdx !== 'OFL-1.1' ||
    family.faces?.length !== 4
  )
    throw new Error('Invalid source');
  const styles = new Set(family.faces.map((face) => `${face.weight}:${face.style}`));
  if (
    styles.size !== 4 ||
    !['400:normal', '400:italic', '700:normal', '700:italic'].every((style) => styles.has(style))
  )
    throw new Error('Invalid faces');
  const names = {
    '400:normal': 'Regular',
    '400:italic': 'Italic',
    '700:normal': 'Bold',
    '700:italic': 'BoldItalic',
  };
  for (const face of family.faces) {
    const name = names[`${face.weight}:${face.style}`];
    if (
      face.renderer?.sourcePath !== `fonts/ttf/LexicaUltralegible-${name}.ttf` ||
      face.web?.sourcePath !== `fonts/web/LexicaUltralegible-${name}.woff2` ||
      !face.renderer.path.endsWith('.ttf') ||
      !face.web.path.endsWith('.woff2')
    )
      throw new Error('Invalid face files');
  }
  if (family.license.sourcePath !== 'LICENSE.txt' || !family.license.path.endsWith('.txt'))
    throw new Error('Invalid license');
  const assets = [family.license, ...family.faces.flatMap((face) => [face.renderer, face.web])];
  const paths = new Set();
  for (const asset of assets) {
    if (
      !SHA.test(asset.sha256) ||
      !PATH.test(asset.path) ||
      !asset.path.includes(asset.sha256) ||
      !Number.isSafeInteger(asset.size) ||
      asset.size < 1 ||
      asset.size > 8 * 1024 * 1024 ||
      !/^(?:LICENSE\.txt|fonts\/(?:ttf|web)\/LexicaUltralegible-(?:Regular|Italic|Bold|BoldItalic)\.(?:ttf|woff2))$/.test(
        asset.sourcePath
      ) ||
      paths.has(asset.path)
    )
      throw new Error(`Invalid asset: ${asset.path}`);
    paths.add(asset.path);
  }
  return {
    catalog,
    family,
    assets,
    bytes,
    manifestPath: `catalogs/lexica-ultralegible/1.0.0/${hash(bytes)}.json`,
  };
}

async function bounded(response, limit) {
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const chunks = [];
  let size = 0;
  for await (const chunk of response.body) {
    size += chunk.length;
    if (size > limit) throw new Error('Asset exceeds expected size');
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks);
}

export async function prepare(directory, fetchImpl = fetch) {
  const { family, assets, bytes, manifestPath } = await readCatalog();
  const objects = new Map();
  for (const asset of assets) {
    const response = await fetchImpl(`${SOURCE}${family.sourceCommit}/${asset.sourcePath}`, {
      redirect: 'error',
      signal: AbortSignal.timeout(60000),
    });
    const data = await bounded(response, asset.size);
    if (data.length !== asset.size || hash(data) !== asset.sha256)
      throw new Error(`Source integrity mismatch: ${asset.sourcePath}`);
    objects.set(asset.path, data);
  }
  objects.set(manifestPath, bytes);
  for (const [path, data] of objects) {
    const file = join(resolve(directory), path);
    await mkdir(dirname(file), { recursive: true });
    await writeFile(file, data);
  }
  return objects;
}

export async function uploadImmutable({ hostname, zone, accessKey, objects, fetchImpl = fetch }) {
  if (
    !/^(?:[a-z0-9-]+\.)?storage\.bunnycdn\.com$/.test(hostname || '') ||
    !/^[a-z0-9-]+$/.test(zone || '') ||
    !accessKey
  )
    throw new Error('Storage configuration is incomplete');
  for (const [path, bytes] of objects) {
    const basename = path.split('/').at(-1);
    if (
      !(
        PATH.test(path) || /^catalogs\/lexica-ultralegible\/1\.0\.0\/[0-9a-f]{64}\.json$/.test(path)
      ) ||
      !basename.startsWith(hash(bytes))
    )
      throw new Error(`Invalid immutable object: ${path}`);
  }
  for (const [path, bytes] of objects) {
    const url = `https://${hostname}/${zone}/${path}`;
    const get = () =>
      fetchImpl(url, {
        headers: { AccessKey: accessKey },
        redirect: 'error',
        signal: AbortSignal.timeout(60000),
      });
    const existing = await get();
    if (existing.status === 200) {
      if (!(await bounded(existing, bytes.length)).equals(bytes))
        throw new Error(`Existing object differs: ${path}`);
      continue;
    }
    await existing.body?.cancel();
    if (existing.status !== 404) throw new Error(`Storage lookup failed: HTTP ${existing.status}`);
    const type = path.endsWith('.woff2')
      ? 'font/woff2'
      : path.endsWith('.ttf')
        ? 'font/ttf'
        : path.endsWith('.json')
          ? 'application/json'
          : 'text/plain';
    const written = await fetchImpl(url, {
      method: 'PUT',
      headers: { AccessKey: accessKey, 'Content-Type': type },
      body: bytes,
      redirect: 'error',
      signal: AbortSignal.timeout(60000),
    });
    await written.body?.cancel();
    if (!written.ok) throw new Error(`Storage upload failed: HTTP ${written.status}`);
    if (!(await bounded(await get(), bytes.length)).equals(bytes))
      throw new Error(`Storage read-back mismatch: ${path}`);
  }
}

export async function verifyPublic(base, objects, fetchImpl = fetch) {
  const url = new URL(base);
  if (url.protocol !== 'https:' || url.username || url.password || url.search || url.hash)
    throw new Error('Public base must be an HTTPS URL');
  for (const [path, bytes] of objects) {
    const response = await fetchImpl(`${base.replace(/\/$/, '')}/${path}`, {
      headers: { Origin: 'https://example.org' },
      redirect: 'error',
      signal: AbortSignal.timeout(60000),
    });
    const actual = await bounded(response, bytes.length);
    if (!actual.equals(bytes)) throw new Error(`Public integrity mismatch: ${path}`);
    if (response.headers.get('access-control-allow-origin') !== '*')
      throw new Error(`Missing public CORS: ${path}`);
    if (
      !/\bimmutable\b/.test(response.headers.get('cache-control') || '') ||
      !/\bmax-age=31536000\b/.test(response.headers.get('cache-control') || '')
    )
      throw new Error(`Missing immutable cache: ${path}`);
    const expected = path.endsWith('.woff2')
      ? 'font/woff2'
      : path.endsWith('.ttf')
        ? 'font/ttf'
        : path.endsWith('.json')
          ? 'application/json'
          : 'text/plain';
    if ((response.headers.get('content-type') || '').split(';')[0] !== expected)
      throw new Error(`Incorrect content type: ${path}`);
  }
}
