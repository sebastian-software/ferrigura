/** Host-side, opt-in catalog resolution. The renderer itself never accesses the network. */
import { createHash, randomUUID } from 'node:crypto';
import { constants } from 'node:fs';
import { link, lstat, mkdir, open, unlink, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const DEFAULT_HASH = '33a0a2fac7f3c0f43e2b38ef79d482e3cc4c75fc3730e241c28a73bef79eeaa8';
const DEFAULT_BASE = 'https://assets.sebastian-software.com/ferrigura/';
const DEFAULT_CATALOG = {
  manifestUrl: `${DEFAULT_BASE}catalogs/fonts/1.0.0/${DEFAULT_HASH}.json`,
  manifestSha256: DEFAULT_HASH,
  assetBase: DEFAULT_BASE,
};
const HASH = /^[a-f0-9]{64}$/;
const MAX_MANIFEST = 2 * 1024 * 1024;
const MAX_FONT = 16 * 1024 * 1024;
const fold = (value) => value.replace(/[A-Z]/g, (letter) => letter.toLowerCase());

function digest(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}
function checkedHash(value, label) {
  if (typeof value !== 'string' || !HASH.test(value))
    throw new TypeError(`${label} must be a lowercase SHA-256`);
  return value;
}
function originOf(value) {
  let url;
  try {
    url = new URL(value);
  } catch {
    throw new TypeError('manifestUrl must be an absolute URL');
  }
  if (
    url.username ||
    url.password ||
    url.search ||
    url.hash ||
    !(
      url.protocol === 'https:' ||
      (url.protocol === 'http:' && ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname))
    )
  ) {
    throw new TypeError(
      'manifestUrl must use HTTPS or loopback HTTP and contain no credentials, query or fragment'
    );
  }
  return url.origin;
}
function checkedUrl(value, origin, label) {
  let url;
  try {
    url = new URL(value);
  } catch {
    throw new TypeError(`${label} must be an absolute URL`);
  }
  if (url.origin !== origin || url.username || url.password || url.hash)
    throw new TypeError(`${label} must use the allowed origin`);
  return url;
}
function assetUrl(base, path) {
  if (
    typeof path !== 'string' ||
    !/^[A-Za-z0-9._/-]+$/.test(path) ||
    !path.startsWith('fonts/') ||
    !/\.(?:ttf|otf)$/.test(path) ||
    path.startsWith('/') ||
    path.split('/').some((part) => !part || part === '.' || part === '..')
  ) {
    throw new TypeError('catalog font path must be a safe relative path');
  }
  const url = new URL(path, base);
  if (url.origin !== base.origin || !url.pathname.startsWith(base.pathname))
    throw new TypeError('catalog font path escapes asset base');
  return url;
}
async function verifiedBytes({ cacheDir, hash, size, limit, url, offline }) {
  const dir = join(cacheDir, 'sha256');
  const path = join(dir, hash);
  let handle;
  try {
    const entry = await lstat(path);
    if (!entry.isFile()) throw new Error(`cached object ${hash} is not a regular file`);
    handle = await open(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
    const info = await handle.stat();
    if (!info.isFile()) throw new Error(`cached object ${hash} is not a regular file`);
    if (info.size > limit || (size != null && info.size !== size))
      throw new Error(`cached object ${hash} has the wrong size`);
    const buffer = Buffer.alloc(info.size + 1);
    let used = 0;
    while (used < buffer.length) {
      const result = await handle.read(buffer, used, buffer.length - used, used);
      if (result.bytesRead === 0) break;
      used += result.bytesRead;
    }
    if (used !== info.size) throw new Error(`cached object ${hash} changed size`);
    const bytes = buffer.subarray(0, used);
    if (digest(bytes) !== hash) throw new Error(`cached object ${hash} has the wrong SHA-256`);
    return bytes;
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  } finally {
    await handle?.close();
  }
  if (offline) throw new Error(`offline cache miss for ${hash}`);
  const response = await fetch(url, { redirect: 'manual', signal: AbortSignal.timeout(30_000) });
  if (!response.ok) {
    await response.body?.cancel().catch(() => {});
    throw new Error(`font catalog request failed: HTTP ${response.status} for ${url}`);
  }
  if (!response.body) throw new Error(`font catalog response has no body for ${url}`);
  const chunks = [];
  let total = 0;
  const reader = response.body.getReader();
  try {
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      total += value.byteLength;
      if (total > limit || (size != null && total > size))
        throw new Error(`font catalog response is too large for ${url}`);
      chunks.push(value);
    }
  } finally {
    try {
      await reader.cancel().catch(() => {});
    } finally {
      reader.releaseLock();
    }
  }
  const bytes = Buffer.concat(chunks, total);
  if ((size != null && total !== size) || digest(bytes) !== hash)
    throw new Error(`font catalog object failed size or SHA-256 verification for ${url}`);
  await mkdir(dir, { recursive: true });
  const temporary = join(dir, `.${hash}.${randomUUID()}.tmp`);
  try {
    await writeFile(temporary, bytes, { flag: 'wx' });
    try {
      // Publish without replacing a concurrent writer's open file (Windows rejects that rename).
      await link(temporary, path);
    } catch (error) {
      if (error.code !== 'EEXIST') throw error;
      // Another process won the race. Never trust or overwrite its object without verifying it.
      return await verifiedBytes({ cacheDir, hash, size, limit, url, offline: true });
    }
  } finally {
    await unlink(temporary).catch((error) => {
      if (error.code !== 'ENOENT') throw error;
    });
  }
  return bytes;
}

/** Fetch and verify explicitly selected catalog families; returns inputs for Renderer. */
export async function resolveCatalogFonts(options) {
  if (!options || typeof options.cacheDir !== 'string' || !options.cacheDir)
    throw new TypeError('cacheDir is required');
  if (!Array.isArray(options.families) || options.families.length === 0)
    throw new TypeError('at least one family is required');
  if (options.offline != null && typeof options.offline !== 'boolean')
    throw new TypeError('offline must be a boolean');
  const catalog = options.catalog ?? DEFAULT_CATALOG;
  const origin = originOf(catalog.manifestUrl);
  const manifestUrl = checkedUrl(catalog.manifestUrl, origin, 'manifestUrl');
  const base = checkedUrl(catalog.assetBase, origin, 'assetBase');
  if (!base.pathname.endsWith('/') || base.search)
    throw new TypeError('assetBase must end in / and have no query');
  const manifestHash = checkedHash(catalog.manifestSha256, 'manifestSha256');
  const manifest = await verifiedBytes({
    cacheDir: options.cacheDir,
    hash: manifestHash,
    limit: MAX_MANIFEST,
    url: manifestUrl,
    offline: options.offline,
  });
  let parsed;
  try {
    parsed = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(manifest));
  } catch {
    throw new Error('font catalog is not valid UTF-8 JSON');
  }
  if (parsed?.schemaVersion !== 1 || !Array.isArray(parsed.families))
    throw new Error('unsupported font catalog schema');
  const seenFamilies = new Set();
  for (const family of parsed.families) {
    if (
      typeof family?.family !== 'string' ||
      !family.family.trim() ||
      /[\u0000-\u001f\u007f]/.test(family.family) ||
      typeof family.version !== 'string' ||
      !family.version.trim() ||
      !Array.isArray(family.faces)
    )
      throw new Error('invalid catalog family');
    const key = fold(family.family);
    if (seenFamilies.has(key)) throw new Error(`duplicate catalog family ${family.family}`);
    seenFamilies.add(key);
    const designs = new Set();
    for (const face of family.faces) {
      if (
        !Number.isInteger(face.weight) ||
        face.weight < 1 ||
        face.weight > 1000 ||
        !['normal', 'italic'].includes(face.style)
      )
        throw new Error(`invalid face for ${family.family}`);
      const design = `${face.weight}/${face.style}`;
      if (designs.has(design)) throw new Error(`duplicate face ${design} for ${family.family}`);
      designs.add(design);
      checkedHash(face.renderer?.sha256, 'renderer SHA-256');
      if (
        !Number.isSafeInteger(face.renderer?.size) ||
        face.renderer.size < 1 ||
        face.renderer.size > MAX_FONT
      )
        throw new Error(`invalid renderer size for ${family.family}`);
      assetUrl(base, face.renderer.path);
      const file = face.renderer.path.split('/').at(-1);
      if (![`${face.renderer.sha256}.ttf`, `${face.renderer.sha256}.otf`].includes(file))
        throw new Error(`renderer path is not content-addressed for ${family.family}`);
    }
  }
  const aliases = new Set();
  const chosen = options.families.map((selection) => {
    const requested = typeof selection === 'string' ? selection : selection?.family;
    const alias = typeof selection === 'string' ? selection : selection?.alias;
    if (
      typeof requested !== 'string' ||
      !requested.trim() ||
      typeof alias !== 'string' ||
      !alias.trim() ||
      /[\u0000-\u001f\u007f]/.test(alias)
    )
      throw new TypeError('each selection needs a family and a valid alias');
    const key = fold(alias);
    if (aliases.has(key)) throw new TypeError(`duplicate font alias ${alias}`);
    aliases.add(key);
    const family = parsed.families.find((entry) => fold(entry.family) === fold(requested));
    if (!family || family.faces.length === 0)
      throw new Error(`font family ${JSON.stringify(requested)} is not in the catalog`);
    return { family, alias };
  });
  const requestedFallbacks = options.fallbacks ?? [];
  if (!Array.isArray(requestedFallbacks)) throw new TypeError('fallbacks must be an array');
  const fallbackKeys = new Set();
  const fallbacks = requestedFallbacks.map((name) => {
    if (typeof name !== 'string' || !name.trim())
      throw new TypeError('fallbacks must be registered aliases');
    const key = fold(name);
    const entry = chosen.find(({ alias }) => fold(alias) === key);
    if (!entry) throw new TypeError(`fallback ${JSON.stringify(name)} is not a selected alias`);
    if (fallbackKeys.has(key)) throw new TypeError(`duplicate fallback alias ${name}`);
    fallbackKeys.add(key);
    return entry.alias;
  });
  const fonts = [];
  for (const { family, alias } of chosen)
    for (const face of family.faces) {
      const data = await verifiedBytes({
        cacheDir: options.cacheDir,
        hash: face.renderer.sha256,
        size: face.renderer.size,
        limit: MAX_FONT,
        url: assetUrl(base, face.renderer.path),
        offline: options.offline,
      });
      fonts.push({ family: alias, weight: face.weight, style: face.style, data });
    }
  return { fonts, fallbacks };
}
